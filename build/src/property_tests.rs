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
    rebuilder::{CachingMTimeRebuilder, Dirtiness, DirtyCache},
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

impl DirtyCache for MapMTimeState {
    fn dirtiness(&self, key: Key) -> std::io::Result<Dirtiness> {
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
        mtimes.insert(Key::Path(b"foo.c".to_vec().into()), mtime_a);
        mtimes.insert(Key::Path(b"foo".to_vec().into()), mtime_b);
        let state = MapMTimeState { map: RefCell::new(mtimes) };
        let rebuilder = CachingMTimeRebuilder::new(state);
        let maybe_task =
        rebuilder.build(Key::Path(b"foo".to_vec().into()), None, &Task {
            dependencies: vec![Key::Path(b"foo.c".to_vec().into())],
                            order_dependencies: vec![],
            variant: TaskVariant::Command("cc -c foo.c".to_owned()),
        });
        match (mtime_a, mtime_b) {
            (Dirtiness::Modified(a), Dirtiness::Modified(b)) => {
                let maybe_task = maybe_task.expect("not an error");
                if a < b {
                    let _ = maybe_task.expect_none("if input is older, no rebuild expected");
                } else {
                    let _ = maybe_task.expect("if input is newer, rebuild expected");
                }
            },
            (Dirtiness::Modified(_a), _) => {
                let _ = maybe_task.expect("not a failure since if input is modified we need to consider rebuilding").expect("should rebuild");
            },
            (Dirtiness::DoesNotExist, _) => { let _ = maybe_task.expect_err("missing input"); },
            (Dirtiness::Dirty, _) => { let _ = maybe_task.expect("not an error").expect("if input is dirty, need to rebuild"); },
            (Dirtiness::Clean, _) => { panic!("Should never happen"); },
        }
    }
}
