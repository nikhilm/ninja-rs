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

use proptest::prelude::*;

use super::{
    interface::Rebuilder,
    rebuilder::{Dirtiness, MTimeRebuilder, MTimeStateI},
};
use crate::task::{Key, Task, TaskVariant};
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

    fn mark_dirty(&self, _key: Key, _is_dirty: bool) {
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
        rebuilder.build(Key::Single(b"foo".to_vec()), None, &Task {
            dependencies: vec![Key::Single(b"foo.c".to_vec())],
                            order_dependencies: vec![],
            variant: TaskVariant::Command("cc -c foo.c".to_owned()),
        });
        match (mtime_a, mtime_b) {
            (Dirtiness::Modified(a), Dirtiness::Modified(b)) => {
                let maybe_task = maybe_task.expect("not an error");
                if a < b {
                    assert!(!maybe_task.is_command(), "if input is older, no rebuild expected");
                } else {
                    assert!(maybe_task.is_command(), "if input is newer, rebuild expected");
                }
            },
            (Dirtiness::Modified(_a), _) => { assert!(maybe_task.expect("not a failure since if input is modified we need to consider rebuilding").is_command(), "should rebuild"); },
            (Dirtiness::DoesNotExist, _) => { maybe_task.expect_err("missing input"); },
            (Dirtiness::Dirty, _) => { assert!(maybe_task.expect("not an error").is_command(), "if input is dirty, need to rebuild"); },
            (Dirtiness::Clean, _) => { panic!("Should never happen"); },
        }
    }
}
