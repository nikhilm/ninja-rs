use std::collections::{hash_map::Entry, HashMap};

pub type PathRef = usize;

// Our "Build" paper abstraction breaks down here as we start talking about paths, so this is an
// area to revisit.
#[derive(Debug)]
struct PathNode {
    path: Vec<u8>,
}

#[derive(Debug)]
pub struct PathCache {
    nodes: Vec<PathNode>,
    // Not clear yet if the key should be &[u8] or OsString.
    map: HashMap<Vec<u8>, PathRef>,
}

// Rough translation of HashMap entry API to be more ergonomic.
pub enum InsertResult {
    AlreadyExists(PathRef),
    Inserted(PathRef),
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

    // The same path ends up returning a re-used noderef.
    // the only thing that needs to check for collisions is the parser, where it may want to
    // complain for output nodes
    pub fn insert<P: Into<Vec<u8>>>(&mut self, path: P) -> InsertResult {
        // TODO: canonicalization
        // TODO: Sucks to clone, particularly if we hit the occupied case.
        let p = path.into();
        let clone = p.clone();
        match self.map.entry(p) {
            Entry::Occupied(e) => InsertResult::AlreadyExists(*e.get()),
            Entry::Vacant(e) => {
                self.nodes.push(PathNode { path: clone });
                let idx = self.nodes.len() - 1;
                e.insert(idx);
                InsertResult::Inserted(idx)
            }
        }
    }

    pub fn insert_and_get<P: Into<Vec<u8>>>(&mut self, path: P) -> PathRef {
        match self.insert(path) {
            InsertResult::AlreadyExists(r) => r,
            InsertResult::Inserted(r) => r,
        }
    }

    pub fn get(&self, rf: PathRef) -> &[u8] {
        &self.nodes[rf].path
    }

    // Should this in-place edit?
    // fn canonicalize(path: &[u8]) -> &[u8] {}
}
