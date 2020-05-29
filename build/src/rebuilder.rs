use super::TaskResult;
use ninja_tasks::{Key, Task};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    ffi::OsStr,
    fs::metadata,
    path::Path,
    string::FromUtf8Error,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

use std::os::unix::ffi::OsStrExt;

use crate::{
    disk_interface::DiskInterface,
    interface::{BuildTask, Rebuilder},
    task::{CommandTask, NoopTask},
};

/**
 * Ninja's modification status is slightly more complex than simply comparing mtimes. This is even
 * without bringing dynamic dependencies and the build log into the picture. It falls out of the
 * behavior of the phony rule. Or any rule really, but phony is commonly used.
 *
 * Ninja will fail with this build edge, because file_does_not_exist is missing and is a source
 * file.
 * ```ninja
 * build some_file: some_rule file_does_not_exist
 * ```
 *
 * Adding a phony rule is enough to make this a valid build description:
 * ```ninja
 * build file_does_not_exist: phony
 * build some_file: some_rule file_does_not_exist
 * ```
 *
 * Even though the state of the file system has not changed, ninja sees the phony rule and declares
 * the `file_does_not_exist` output as already dirty.
 *
 * In Ninja, this behavior falls out of Builder::AddTarget, DependencyScan::RecomputeDirty and
 * Plan::AddSubTarget.
 *
 * In case 1, RecomputeDirty marks file_does_not_exist as dirty and some_file as dirty. Then,
 * some_file is added to the plan, and all its inputs are also added. At this point we hit the
 * "input does not have any incoming edge, but is dirty, which means a source file does not exist"
 * and Ninja fails.
 *
 * In case 2, RecomputeDirty marks file_does_not_exist as dirty and some_file as dirty. As we
 * proceed to add file_does_not_exist to the plan, it does have an incoming edge, so it does not
 * fail.
 *
 * Basically, if a file is an output, then it can _not exist_ and other edges are allowed to depend
 * on it without any kind of existence check. It is always just considered dirty.
 *
 * Now, in the a la carte model, the rebuilder cannot really look at transitive dependencies for a
 * task. That is, we have no notion of AddSubTarget. We do know that the scheduler is expected to
 * preserve a topological order. So the invariant that the rebuilder will see all dependencies before
 * the dependent is still preserved. So to implement similar behavior, we need to mark "oh this was
 * an output for a build edge". If a file is known to be an output of another edge, it is allowed
 * to be missing for state, and we will consider the dependent as also dirty.
 *
 * It is unclear how to model this in terms of the information MTimeState should preserve. It can
 * either maintain a mtime cache + a dirty cache, or a mtime cache + seen outputs cache. It would
 * be nice to represent them nicely so that we don't end up in invalid states when going through a
 * fairly complex build function.
 *
 */

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Dirtiness {
    Dirty,
    // This means we already looked and the file does not exist, unlike Option<Dirtiness> which
    // means we haven't looked yet.
    DoesNotExist,
    Modified(SystemTime),
}

#[derive(Debug)]
pub struct MTimeState<Disk>
where
    Disk: DiskInterface,
{
    // This Key abstraction is unnatural because most places don't care about multi-keys.
    dirty: RefCell<HashMap<Key, Dirtiness>>,
    disk: Disk,
}

impl<Disk> MTimeState<Disk>
where
    Disk: DiskInterface,
{
    pub fn new(disk: Disk) -> Self {
        MTimeState {
            disk,
            dirty: Default::default(),
        }
    }

    pub fn modified(&self, key: Key) -> std::io::Result<Dirtiness> {
        match self.dirty.borrow_mut().entry(key.clone()) {
            Entry::Occupied(e) => Ok(*e.get()),
            Entry::Vacant(_) => {
                // Multi keys can only have modified called on them if a previous run marked them
                // as dirty.
                assert!(key.is_single());
                self.disk
                    .modified(OsStr::from_bytes(key.as_bytes()))
                    .map(Dirtiness::Modified)
                    .or_else(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            Ok(Dirtiness::DoesNotExist)
                        } else {
                            Err(e)
                        }
                    })
            }
        }
    }

    pub fn mark_dirty(&self, key: Key) {
        self.dirty.borrow_mut().insert(key, Dirtiness::Dirty);
    }
}

#[derive(Debug)]
pub struct MTimeRebuilder<Disk>
where
    Disk: DiskInterface,
{
    mtime_state: MTimeState<Disk>,
}

impl<Disk> MTimeRebuilder<Disk>
where
    Disk: DiskInterface,
{
    pub fn new(mtime_state: MTimeState<Disk>) -> Self {
        Self { mtime_state }
    }
}

#[derive(Error, Debug)]
pub enum RebuilderError {
    #[error("utf-8 error")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("'{input}', needed by '{output}', missing and no known rule to make it")]
    MissingInput { output: String, input: String },
    #[error("error looking up mtime")]
    IOError(#[from] std::io::Error),
}

impl<Disk> Rebuilder<Key, TaskResult, (), RebuilderError> for MTimeRebuilder<Disk>
where
    Disk: DiskInterface,
{
    fn build(
        &self,
        key: Key,
        _current_value: TaskResult,
        task: &Task,
    ) -> Result<Box<dyn BuildTask<(), TaskResult> + Send>, RebuilderError> {
        // This function obviously needs a lot of error handling.
        // Only returns the command task if required, otherwise a dummy.

        let outputs_dirty: Dirtiness = match key.clone() {
            Key::Single(_) => self.mtime_state.modified(key.clone())?,
            Key::Multi(keys) => {
                assert!(keys.len() > 1);
                // Non-empty multi-keys really should be asserted elsewhere.
                // We actually want something like sorbet's "values have assertable conditions in
                // debug mode".
                keys.iter()
                    .try_fold(
                        None,
                        |so_far, current_key| -> Result<Option<Dirtiness>, RebuilderError> {
                            let dirty = self.mtime_state.modified(current_key.clone())?;
                            if so_far.is_none() {
                                Ok(Some(dirty))
                            } else {
                                let so_far = so_far.unwrap();
                                Ok(Some(match (so_far, dirty) {
                                    // If we get 2 mtimes, we compare them and return the smaller one.
                                    // Everything else means at least one output is missing or dirty, which
                                    // translates to everything being dirty.
                                    (
                                        Dirtiness::Modified(so_far),
                                        Dirtiness::Modified(this_one),
                                    ) => Dirtiness::Modified(std::cmp::min(so_far, this_one)),
                                    // Dirty wins over everything else.
                                    _ => Dirtiness::Dirty,
                                }))
                            }
                        },
                    )?
                    .expect("non-None because multi-key has at least two elements")
            }
        };

        // Iterate inputs to make sure they exist, regardless of what outputs were determined.
        let dependencies = task.dependencies();
        // Dependencies can either be a single Multi key or a list of Singles.
        let inputs_dirty = if dependencies.len() == 1 && matches!(dependencies[0], Key::Multi(_)) {
            Some(self.mtime_state.modified(dependencies[0].clone())?)
        } else {
            // TODO if debug.
            for dep in dependencies {
                assert!(dep.is_single());
            }
            // We could use iter.any, but that will short circuit and not check every file for
            // existence.
            dependencies.iter().try_fold(
                None,
                |so_far, current_dep| -> Result<Option<Dirtiness>, RebuilderError> {
                    match current_dep {
                        Key::Single(ref dep_bytes) => {
                            let dep_mtime = self.mtime_state.modified(current_dep.clone())?;
                            if dep_mtime == Dirtiness::DoesNotExist {
                                let output = match key.clone() {
                                    Key::Single(_) => String::from_utf8(key.as_bytes().to_vec())?,
                                    Key::Multi(keys) => {
                                        String::from_utf8(keys[0].as_bytes().to_vec())?
                                    }
                                };
                                Err(RebuilderError::MissingInput {
                                    input: String::from_utf8(dep_bytes.clone().to_vec())?,
                                    output,
                                })
                            } else {
                                Ok(if so_far.is_none() {
                                    Some(dep_mtime)
                                } else {
                                    let so_far = so_far.unwrap();
                                    assert_ne!(so_far, Dirtiness::DoesNotExist);
                                    assert_ne!(dep_mtime, Dirtiness::DoesNotExist);
                                    Some(match (so_far, dep_mtime) {
                                        // max of inputs, so we can check if newest input is older than
                                        // oldest output.
                                        (
                                            Dirtiness::Modified(so_far),
                                            Dirtiness::Modified(dep_mtime),
                                        ) => Dirtiness::Modified(std::cmp::max(so_far, dep_mtime)),
                                        _ => Dirtiness::Dirty,
                                    })
                                })
                            }
                        }
                        _ => unreachable!(),
                    }
                },
            )?
        };
        let dirty = if let Dirtiness::Modified(output_mtime) = outputs_dirty {
            if let Some(inputs_dirty) = inputs_dirty {
                match inputs_dirty {
                    Dirtiness::Dirty => true,
                    Dirtiness::DoesNotExist => unreachable!(),
                    Dirtiness::Modified(input_mtime) => input_mtime > output_mtime,
                }
            } else {
                false
            }
        } else {
            true
        };

        if dirty {
            self.mtime_state.mark_dirty(key.clone());
        }

        eprintln!("{} dirty? {}", &key, dirty);
        if dirty && task.is_command() {
            // TODO: actually need some return type that can failure to run this task if the
            // dependency is not available.
            // may want different response based on dep being source vs intermediate. for
            // intermediate, whatever should've produced it will fail and have the error message.
            // So fail with not found if not a known output.
            Ok(Box::new(CommandTask::new(task.command().unwrap().clone())))
        } else {
            // TODO: current value?
            Ok(Box::new(NoopTask {}))
        }
    }
}

#[cfg(test)]
mod test {
    use insta::assert_display_snapshot;
    use std::{
        io::{Error, ErrorKind, Result},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use ninja_tasks::*;

    // We need enough flexibility that we can test mock paths with mock mtimes and simulate the
    // relevant results we want. It would be nice to feed that to the actual rebuilder build fn.
    #[test]
    fn test_basic() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime> {
                if p.as_ref() == Path::new("foo.c") {
                    Ok(UNIX_EPOCH.checked_add(Duration::from_secs(100)).unwrap())
                } else {
                    Err(Error::new(ErrorKind::NotFound, "mock not found"))
                }
            }
        }

        // TODO: Starting point for making the rebuilder depend more on the state and "fixing"
        // the state to be more trait-y as well as not always depend on IO.
        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        let task = Task {
            dependencies: vec![Key::Single(b"foo.c".to_vec())],
            variant: TaskVariant::Command("cc -c foo.c".to_owned()),
        };
        eprintln!("Task is command {}", task.is_command());
        let task = rebuilder
            .build(Key::Single(b"foo.o".to_vec()), TaskResult {}, &task)
            .expect("valid task");
        assert!(task.is_command());
    }

    /// A rule where the input does not exist should fail.
    #[test]
    fn test_input_does_not_exist() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, _: P) -> Result<SystemTime> {
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
            }
        }

        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        // What we really want in this test is that if the input is not itself an output of
        // something else, then we want to error if it does not exist.
        // What we know: inputs MUST exist by the time a task is executed, because the scheduler
        // guarantees it.
        // that satisfies erroring out when inputs do not exist.
        // It probably won't satisfy "oh if this is wrapped in a phony". Actually it can, by
        // marking the mtimestate with some marker when the output did not exist. essentially a
        // "dirtiness" state instead of a mtimestate.
        let task = rebuilder.build(
            Key::Single(b"phony_user".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_err());
        match task {
            Err(e) => {
                assert_display_snapshot!(e);
            }
            _ => assert!(false, "Expected error"),
        }

        let task = rebuilder.build(
            Key::Single(b"phony_user".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
                variant: TaskVariant::Command("whatever".to_string()),
            },
        );
        assert!(task.is_err());
        match task {
            Err(e) => {
                assert_display_snapshot!(e);
            }
            _ => assert!(false, "Expected error"),
        }
    }

    #[test]
    fn test_input_does_not_exist_multiple_out_error_message() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, _: P) -> Result<SystemTime> {
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
            }
        }

        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        let task = Task {
            dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
            variant: TaskVariant::Retrieve,
        };
        let task = rebuilder.build(
            Key::Multi(vec![
                Key::Single(b"phony_user".to_vec()),
                Key::Single(b"phony_user2".to_vec()),
            ]),
            TaskResult {},
            &task,
        );
        assert!(task.is_err());
        match task {
            Err(e) => {
                assert_display_snapshot!(e);
            }
            _ => assert!(false, "Expected error"),
        }
    }

    #[test]
    fn test_phony_input() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, _: P) -> Result<SystemTime> {
                // This test should not hit disk.
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
            }
        }

        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        let task = rebuilder.build(
            Key::Single(b"phony_target_that_does_not_exist".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_ok());

        // Since the above marked the output as phony/dirty, this one should not fail because the
        // cache should treat it as dirty.
        let task = rebuilder.build(
            Key::Single(b"phony_user".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_ok());
    }
}
