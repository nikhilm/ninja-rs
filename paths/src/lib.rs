use std::{
    collections::HashMap,
};

#[derive(Debug)]
struct PathNode {}

#[derive(Debug)]
pub struct PathCache {
    nodes: Vec<PathNode>,
    // Not clear yet if the key should be &[u8] or OsString.
    map: HashMap<Vec<u8>, usize>,
}

// We want access (entries) from PathCache to be tied to the path cache's lifetime. in addition,
// should not be able to pass a pathcache entry from one to another, if possible.
// i.e. don't want to just return a usize, and PathNode should probably never escape from the
// cache. Instead hand out references.

// It is possible for path canonicalization to never need to touch disk, if we assume 2 things:
// 1. There is always one "entry point" for ninja - which is the build.ninja or another file the
//    command is invoked with.
// 2. All other ninja files reachable from this file, when they want to refer to the same file on
//    disk, use the relevant `..` or .ninja file relative manipulations to do so.
// This is a reasonable expectation from .ninja file authors since that is how they are expected to
// refer to the same files.

impl PathCache {
    pub fn new() -> PathCache {
        PathCache {
            nodes: vec![],
            map: HashMap::new(),
        }
    }

    // Should this in-place edit?
    // fn canonicalize(path: &[u8]) -> &[u8] {}
}
