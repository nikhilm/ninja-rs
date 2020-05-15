use core::fmt::Debug;
use ninja_tasks::{Task, Tasks};

pub trait BuildTask<State, V> {
    fn run(&self, state: &mut State) -> V;
}

impl<State, V> Debug for dyn BuildTask<State, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BuildTask{{}}")
    }
}

pub trait Rebuilder<K, V, State> {
    fn build(&self, key: K, current_value: V, task: Task) -> Box<dyn BuildTask<State, V>>;
}

pub trait Scheduler<K, V, State> {
    fn schedule(
        &self,
        rebuilder: &dyn Rebuilder<K, V, State>,
        state: State,
        tasks: Tasks,
        start: Vec<K>,
    );
    fn schedule_externals(
        &self,
        rebuilder: &dyn Rebuilder<K, V, State>,
        state: State,
        tasks: Tasks,
    );
}
