/*
 * Copyright 2020 Nikhil Marathe <nsm.nikhil@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    string::FromUtf8Error,
    time::SystemTime,
};

use ninja_metrics::scoped_metric;
use thiserror::Error;

use crate::{
    build_task::{CommandTask, CommandTaskResult, NinjaTask},
    disk_interface::DiskInterface,
    interface::Rebuilder,
    task::{Key, Task},
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
    // We need clean to handle a very specific case, different from non-existence in the cache.
    // Invariant: It does not make any sense to request stat() on a Key::Multi, which means the
    // cache should always have an entry for a Key::Multi by the time it is encountered.
    // This requires us to mark the Key::Multi when it is the output instead of an input.
    // If we only have Dirty, we can't mark the cache when the key is actually not dirty because
    // the dirtiness conditions for the edge producing the Multi were not satisfied. In that case
    // we mark it as clean.
    Clean,
    Dirty,
    // This means we already looked and the file does not exist, unlike Option<Dirtiness> which
    // means we haven't looked yet.
    DoesNotExist,
    Modified(SystemTime),
}

pub trait DirtyCache {
    fn dirtiness(&self, key: Key) -> std::io::Result<Dirtiness>;
    fn mark_dirty(&self, key: Key, is_dirty: bool);
}

#[derive(Debug)]
pub struct DiskDirtyCache<Disk>
where
    Disk: DiskInterface,
{
    // This Key abstraction is unnatural because most places don't care about multi-keys.
    dirty: RefCell<HashMap<Key, Dirtiness>>,
    disk: Disk,
}

impl<Disk> DiskDirtyCache<Disk>
where
    Disk: DiskInterface,
{
    pub fn new(disk: Disk) -> Self {
        DiskDirtyCache {
            disk,
            dirty: Default::default(),
        }
    }
}

impl<Disk> DirtyCache for DiskDirtyCache<Disk>
where
    Disk: DiskInterface,
{
    fn dirtiness(&self, key: Key) -> std::io::Result<Dirtiness> {
        match self.dirty.borrow_mut().entry(key.clone()) {
            Entry::Occupied(e) => Ok(*e.get()),
            Entry::Vacant(entry) => match key {
                Key::Path(key) => {
                    scoped_metric!("mtime_state_insert");
                    let inserted = entry.insert(
                        self.disk
                            .modified(OsStr::from_bytes(key.as_bytes()))
                            .map(Dirtiness::Modified)
                            .or_else(|e| {
                                if e.kind() == std::io::ErrorKind::NotFound {
                                    Ok(Dirtiness::DoesNotExist)
                                } else {
                                    Err(e)
                                }
                            })?,
                    );
                    Ok(*inserted)
                }
                Key::Multi(_) => {
                    panic!("Cannot mtime a multi-key. Did you forget to mark it as dirty to ensure it is in the cache?");
                }
            },
        }
    }

    fn mark_dirty(&self, key: Key, is_dirty: bool) {
        // Marking as Clean only makes sense for multi-keys. For single-keys that represent
        // filesystem resources, they are either dirty or need to be consulted in the cache in the
        // future.
        if is_dirty || key.is_multi() {
            self.dirty.borrow_mut().insert(
                key,
                if is_dirty {
                    Dirtiness::Dirty
                } else {
                    Dirtiness::Clean
                },
            );
        }
    }
}

#[derive(Debug)]
pub struct CachingMTimeRebuilder<Cache>
where
    Cache: DirtyCache,
{
    mtime_state: Cache,
}

impl<Cache> CachingMTimeRebuilder<Cache>
where
    Cache: DirtyCache,
{
    pub fn new(mtime_state: Cache) -> Self {
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

impl<Cache> Rebuilder<Key, CommandTaskResult> for CachingMTimeRebuilder<Cache>
where
    Cache: DirtyCache,
{
    type Error = RebuilderError;
    type Task = dyn NinjaTask;

    fn build(
        &self,
        key: Key,
        _unused: Option<CommandTaskResult>,
        task: &Task,
    ) -> Result<Option<Box<Self::Task>>, Self::Error> {
        let outputs_dirty: Dirtiness = match key.clone() {
            Key::Path(_) => self.mtime_state.dirtiness(key.clone())?,
            Key::Multi(keys) => {
                debug_assert!(keys.len() > 1);
                // Non-empty multi-keys really should be asserted elsewhere.
                // We actually want something like sorbet's "values have assertable conditions in
                // debug mode".
                keys.iter()
                    .try_fold(
                        None,
                        |so_far, current_key| -> Result<Option<Dirtiness>, RebuilderError> {
                            let dirty =
                                self.mtime_state.dirtiness(Key::Path(current_key.clone()))?;
                            match so_far {
                                None => Ok(Some(dirty)),
                                Some(so_far) => Ok(Some(match (so_far, dirty) {
                                    // If we get 2 mtimes, we compare them and return the smaller one.
                                    // Everything else means at least one output is missing or dirty, which
                                    // translates to everything being dirty.
                                    (
                                        Dirtiness::Modified(so_far),
                                        Dirtiness::Modified(this_one),
                                    ) => Dirtiness::Modified(std::cmp::min(so_far, this_one)),
                                    // Dirty wins over everything else.
                                    _ => Dirtiness::Dirty,
                                })),
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
            assert!(task.is_retrieve());
            Some(self.mtime_state.dirtiness(dependencies[0].clone())?)
        } else {
            // TODO if debug.
            for dep in dependencies {
                assert!(dep.is_path());
            }
            // We could use iter.any, but that will short circuit and not check every file for
            // existence.
            dependencies.iter().try_fold(
                None,
                |so_far, current_dep| -> Result<Option<Dirtiness>, RebuilderError> {
                    match current_dep {
                        Key::Path(key_path) => {
                            let dep_mtime = self.mtime_state.dirtiness(current_dep.clone())?;
                            if dep_mtime == Dirtiness::DoesNotExist {
                                let output = match key.clone() {
                                    Key::Path(key) => String::from_utf8(key.as_bytes().to_vec())?,
                                    Key::Multi(keys) => {
                                        String::from_utf8(keys[0].as_bytes().to_vec())?
                                    }
                                };
                                Err(RebuilderError::MissingInput {
                                    input: String::from_utf8(key_path.as_bytes().to_vec())?,
                                    output,
                                })
                            } else {
                                Ok(match so_far {
                                    None => Some(dep_mtime),
                                    Some(so_far) => {
                                        assert_ne!(so_far, Dirtiness::DoesNotExist);
                                        assert_ne!(dep_mtime, Dirtiness::DoesNotExist);
                                        Some(match (so_far, dep_mtime) {
                                            // max of inputs, so we can check if newest input is older than
                                            // oldest output.
                                            (
                                                Dirtiness::Modified(so_far),
                                                Dirtiness::Modified(dep_mtime),
                                            ) => Dirtiness::Modified(std::cmp::max(
                                                so_far, dep_mtime,
                                            )),
                                            _ => Dirtiness::Dirty,
                                        })
                                    }
                                })
                            }
                        }
                        _ => unreachable!(),
                    }
                },
            )?
        };

        // "When these are out of date, the output is not rebuilt until they are built, but changes
        // in order-only dependencies alone do not cause the output to be rebuilt."
        // I feel like this is pretty ambiguous. It can mean:
        // 1. order-only dependencies only affect the scheduler which should put these in the build
        //    graph to influence sequencing, but they don't affect the rebuilder.
        // 2. On the other hand, "when these are out of date" seems to imply that if these are not
        //    out-of-date, then some other effect happens. i.e. why doesn't it just say "the output
        //    is not rebuilt until order-only dependencies are built"?
        // 3. What does "changes to" mean? Changes in the actual set of dependencies (i.e. files
        //    added or removed), or changes to the files themselves?
        //
        // The ninja source code describes order-only deps as "which are needed before the target
        // builds but which don't cause the target to rebuild" which seems to imply (1).

        let dirty = if let Dirtiness::Modified(output_mtime) = outputs_dirty {
            if let Some(inputs_dirty) = inputs_dirty {
                match inputs_dirty {
                    Dirtiness::Clean => false,
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

        self.mtime_state.mark_dirty(key.clone(), dirty);

        if dirty && task.is_command() {
            // TODO: actually need some return type that can failure to run this task if the
            // dependency is not available.
            // may want different response based on dep being source vs intermediate. for
            // intermediate, whatever should've produced it will fail and have the error message.
            // So fail with not found if not a known output.
            Ok(Some(Box::new(CommandTask::new(
                key,
                task.command().unwrap().clone(),
            ))))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use insta::assert_display_snapshot;
    use std::{
        io::{Error, ErrorKind, Result},
        path::Path,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::task::*;

    macro_rules! mocked_rebuilder {
        ($path:ident, $body:expr) => {{
            struct MockDiskInterface {}

            impl DiskInterface for MockDiskInterface {
                fn modified<P: AsRef<Path>>(&self, $path: P) -> Result<SystemTime> {
                    $body
                }
            }

            let mock_disk = MockDiskInterface {};
            let state = $crate::DiskDirtyCache::new(mock_disk);
            $crate::CachingMTimeRebuilder::new(state)
        }};
        ($body:expr) => {
            mocked_rebuilder! {_unused, $body}
        };
    }

    // We need enough flexibility that we can test mock paths with mock mtimes and simulate the
    // relevant results we want. It would be nice to feed that to the actual rebuilder build fn.
    #[test]
    fn test_basic() {
        let rebuilder = mocked_rebuilder! {p,
                if p.as_ref() == Path::new("foo.c") {
                    Ok(UNIX_EPOCH.checked_add(Duration::from_secs(100)).unwrap())
                } else {
                    Err(Error::new(ErrorKind::NotFound, "mock not found"))
                }
        };
        let task = Task {
            dependencies: vec![Key::Path(b"foo.c".to_vec().into())],
            order_dependencies: vec![],
            variant: TaskVariant::Command("cc -c foo.c".to_owned()),
        };
        let _task = rebuilder
            .build(Key::Path(b"foo.o".to_vec().into()), None, &task)
            .expect("valid task")
            .expect("non-none task");
    }

    /// A rule where the input does not exist should fail.
    #[test]
    fn test_input_does_not_exist() {
        let rebuilder = mocked_rebuilder! {
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
        };
        // What we really want in this test is that if the input is not itself an output of
        // something else, then we want to error if it does not exist.
        // What we know: inputs MUST exist by the time a task is executed, because the scheduler
        // guarantees it.
        // that satisfies erroring out when inputs do not exist.
        // It probably won't satisfy "oh if this is wrapped in a phony". Actually it can, by
        // marking the mtimestate with some marker when the output did not exist. essentially a
        // "dirtiness" state instead of a mtimestate.
        let task = rebuilder.build(
            Key::Path(b"phony_user".to_vec().into()),
            None,
            &Task {
                dependencies: vec![Key::Path(
                    b"phony_target_that_does_not_exist".to_vec().into(),
                )],
                order_dependencies: vec![],
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
            Key::Path(b"phony_user".to_vec().into()),
            None,
            &Task {
                dependencies: vec![Key::Path(
                    b"phony_target_that_does_not_exist".to_vec().into(),
                )],
                order_dependencies: vec![],
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
        let rebuilder = mocked_rebuilder! {
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
        };
        let task = Task {
            dependencies: vec![Key::Path(
                b"phony_target_that_does_not_exist".to_vec().into(),
            )],
            order_dependencies: vec![],
            variant: TaskVariant::Retrieve,
        };
        let task = rebuilder.build(
            Key::Multi(
                vec![
                    b"phony_user".to_vec().into(),
                    b"phony_user2".to_vec().into(),
                ]
                .into(),
            ),
            None,
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
        let rebuilder = mocked_rebuilder! {
                // This test should not hit disk.
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
        };
        let task = rebuilder.build(
            Key::Path(b"phony_target_that_does_not_exist".to_vec().into()),
            None,
            &Task {
                dependencies: vec![],
                order_dependencies: vec![],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_ok());

        // Since the above marked the output as phony/dirty, this one should not fail because the
        // cache should treat it as dirty.
        let task = rebuilder.build(
            Key::Path(b"phony_user".to_vec().into()),
            None,
            &Task {
                dependencies: vec![Key::Path(
                    b"phony_target_that_does_not_exist".to_vec().into(),
                )],
                order_dependencies: vec![],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_ok());
    }

    #[test]
    fn test_older_input() {
        let _rebuilder = mocked_rebuilder! {
                        // This test should not hit disk.
                        Err(Error::new(ErrorKind::NotFound, "mock not found"))
        };
        // todo!();
    }

    /*
     * foo -> foo.o -> foo.c
     * but foo is older than foo.o,  while foo.o is newer than foo.c (could happen because a user
     * touched foo.o). This is a regression test to ensure foo is rebuilt.
     * Previously, due to how `mark_dirty` would mark even single-keys as clean, this would fail.
     */
    #[test]
    fn test_clean_chain() {
        let rebuilder = mocked_rebuilder! {p,
                if p.as_ref() == Path::new("foo.c") {
                    Ok(UNIX_EPOCH.checked_add(Duration::from_secs(100)).unwrap())
                } else if p.as_ref() == Path::new("foo.o") {
                    Ok(UNIX_EPOCH.checked_add(Duration::from_secs(1000)).unwrap())
                } else if p.as_ref() == Path::new("foo") {
                    Ok(UNIX_EPOCH.checked_add(Duration::from_secs(500)).unwrap())
                } else {
                    Err(Error::new(ErrorKind::NotFound, "mock not found"))
                }
        };
        let cc_task = Task {
            dependencies: vec![Key::Path(b"foo.c".to_vec().into())],
            order_dependencies: vec![],
            variant: TaskVariant::Command("cc -c foo.c".to_owned()),
        };
        let link_task = Task {
            dependencies: vec![Key::Path(b"foo.o".to_vec().into())],
            order_dependencies: vec![],
            variant: TaskVariant::Command("cc -o foo foo.o".to_owned()),
        };

        // This would previously end up marking foo.o as Clean in the cache.
        let _task = rebuilder
            .build(Key::Path(b"foo.o".to_vec().into()), None, &cc_task)
            .expect("valid task")
            .expect_none("foo.o newer than foo.c");

        let _task = rebuilder
            .build(Key::Path(b"foo".to_vec().into()), None, &link_task)
            .expect("valid task")
            .expect("non-None task");
    }

    #[test]
    fn test_order_dependencies_newer() {
        // TODO: Add a test where order dependencies are newer, but target should not rebuild.
    }
}
