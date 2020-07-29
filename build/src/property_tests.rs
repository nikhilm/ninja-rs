use proptest::prelude::*;

use super::{
    interface::Rebuilder,
    rebuilder::{Dirtiness, MTimeRebuilder, MTimeStateI},
};
use ninja_tasks::{Key, Task, TaskVariant};
use std::{cell::RefCell, collections::HashMap, time::SystemTime};

fn dirtiness_strategy_single() -> impl Strategy<Value = Dirtiness> {
    prop_oneof![
        // Just(Dirtiness::Clean),
        Just(Dirtiness::Dirty),
        Just(Dirtiness::DoesNotExist),
        any::<SystemTime>().prop_map(Dirtiness::Modified),
    ]
}

struct MapMTimeState {
    map: RefCell<HashMap<Key, Dirtiness>>,
}

impl MTimeStateI for MapMTimeState {
    fn modified(&self, key: Key) -> std::io::Result<Dirtiness> {
        if let Some(d) = self.map.borrow().get(&key) {
            Ok(*d)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            ))
        }
    }

    fn mark_dirty(&self, key: Key, is_dirty: bool) {
        // TODO
    }
}

proptest! {
    #[test]
    fn rebuilder_doesnt_crash(mtime_a in dirtiness_strategy_single(), mtime_b in dirtiness_strategy_single()) {
        let mut mtimes = HashMap::new();
        dbg!(mtime_a);
        dbg!(mtime_b);
        mtimes.insert(Key::Single(b"foo.c".to_vec()), mtime_a);
        mtimes.insert(Key::Single(b"foo".to_vec()), mtime_b);
        let state = MapMTimeState { map: RefCell::new(mtimes) };
        let rebuilder = MTimeRebuilder::new(state);
        let maybe_task =
        rebuilder.build(Key::Single(b"foo".to_vec()), &Task {
            dependencies: vec![Key::Single(b"foo.c".to_vec())],
                            order_dependencies: vec![],
            variant: TaskVariant::Command("cc -c foo.c".to_owned()),
        });
        match (mtime_a, mtime_b) {
            (Dirtiness::Modified(a), Dirtiness::Modified(b)) => {
                let maybe_task = maybe_task.expect("not an error");
                if a < b {
                    maybe_task.expect_none("if input is older, no rebuild expected");
                } else {
                    maybe_task.expect("if input is newer, rebuild expected");
                }
            },
            (Dirtiness::Modified(a), _) => { maybe_task.expect("not a failure since if input is modified we need to consider rebuilding").expect("should rebuild"); },
            (Dirtiness::DoesNotExist, _) => { maybe_task.expect_err("missing input"); },
            (Dirtiness::Dirty, _) => { maybe_task.expect("not an error").expect("if input is dirty, need to rebuild"); },
            (Dirtiness::Clean, _) => { panic!("Should never happen"); },
        }
    }
}
