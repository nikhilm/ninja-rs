use core::fmt::Debug;
use ninja_tasks::{Task, Tasks};
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait BuildTask<State, V>
where
{
    // Cannot pass state until we have structured concurrency.
    async fn run(&self, state: &State) -> V;

    #[cfg(test)]
    fn is_command(&self) -> bool {
        false
    }
}

impl<State, V> Debug for dyn BuildTask<State, V>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BuildTask{{}}")
    }
}

pub trait Rebuilder<K, V, State, RebuilderError>
{
    fn build(
        &self,
        key: K,
        // current_value: V,
        task: &Task,
    ) -> Result<Option<Box<dyn BuildTask<State, V>>>, RebuilderError>;
}

pub trait Scheduler<K, V, State, BuildError, RebuilderError>
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
