use super::TaskResult;
use ninja_interface::{BuildTask, Rebuilder};
use ninja_tasks::{Key, Task};
use std::{ffi::OsStr, fs::metadata, os::unix::ffi::OsStrExt};

#[derive(Debug, Default)]
pub struct MTimeState {}

#[derive(Debug, Default)]
pub struct MTimeRebuilder {}
impl Rebuilder<Key, TaskResult, MTimeState> for MTimeRebuilder {
    fn build(
        &self,
        key: Key,
        current_value: TaskResult,
        task: Task,
    ) -> Box<dyn BuildTask<MTimeState, TaskResult>> {
        todo!("Wait");
        /*
                // This function obviously needs a lot of error handling.
                let key = &self.graph[node];
                let mtime = match key {
                    Key::Path(path_ref) => {
                        todo!();
                        /*let path_str: &OsStr = OsStrExt::from_bytes(self.path_cache.get(*path_ref));
                        let path = std::path::Path::new(path_str);
                        if path.exists() {
                            Some(metadata(path).expect("metadata").modified().expect("mtime"))
                        } else {
                            None
                        }*/
                    }
                    Key::Multi(outputs) => {
                        todo!();
                        /*
                        // If the oldest output is older than any input, rebuild.
                        let times: Vec<std::time::SystemTime> = outputs
                            .iter()
                            .filter_map(|path_ref| {
                                let path_str: &OsStr = OsStrExt::from_bytes(self.path_cache.get(*path_ref));
                                let path = std::path::Path::new(path_str);
                                if path.exists() {
                                    Some(metadata(path).expect("metadata").modified().expect("mtime"))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if times.len() < outputs.len() {
                            // At least one output did not exist, so always build.
                            None
                        } else {
                            Some(times.into_iter().min().expect("at least one"))
                        }
                        */
                    }
                };
                let dirty = if mtime.is_none() {
                    true
                } else {
                    let mtime = mtime.unwrap();
                    let mut dependencies = self.graph.neighbors_directed(node, Direction::Outgoing);
                    dependencies.any(|dep| {
                        let dep = &self.graph[dep];
                        match dep {
                            Key::Path(path_ref) => {
                                todo!();
                                /*
                                let path_str: &OsStr = OsStrExt::from_bytes(self.path_cache.get(*path_ref));
                                let dep_mtime = metadata(path_str)
                                    .expect("metadata")
                                    .modified()
                                    .expect("mtime");
                                dep_mtime > mtime
                                */
                            }
                            Key::Multi(_) => {
                                // TODO: assert task is phony.
                                true
                            }
                        }
                    })
                };
                if dirty {
                    // TODO: actually need some return type that can failure to run this task if the
                    // dependency is not available.
                    // may want different response based on dep being source vs intermediate. for
                    // intermediate, whatever should've produced it will fail and have the error message.
                    // So fail with not found if not a known output.
                    task.run();
                }
                TaskResult {}
        */
    }
}
