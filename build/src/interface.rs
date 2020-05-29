use core::fmt::Debug;
use ninja_tasks::{Task, Tasks};

pub trait BuildTask<State, V>
where
    State: Sync,
    V: Send,
{
    fn run(&self, state: &State) -> V;

    #[cfg(test)]
    fn is_command(&self) -> bool {
        false
    }
}

impl<State, V> Debug for dyn BuildTask<State, V>
where
    State: Sync,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BuildTask{{}}")
    }
}

pub trait Rebuilder<K, V, State, RebuilderError>
where
    State: Sync,
{
    fn build(
        &self,
        key: K,
        current_value: V,
        task: &Task,
    ) -> Result<Box<dyn BuildTask<State, V> + Send>, RebuilderError>;
}

pub trait Scheduler<K, V, State, BuildError, RebuilderError>
where
    State: Sync,
{
    fn schedule(
        &self,
        rebuilder: &dyn Rebuilder<K, V, State, RebuilderError>,
        state: State,
        tasks: &Tasks,
        start: Vec<K>,
    ) -> Result<(), BuildError>;
    fn schedule_externals(
        &self,
        rebuilder: &dyn Rebuilder<K, V, State, RebuilderError>,
        state: State,
        tasks: &Tasks,
    ) -> Result<(), BuildError>;
}
