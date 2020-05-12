use core::fmt::Debug;

pub trait Task<V> {
    fn run(&self /*, fetch: Fetch<K, V>*/) -> V;
}

impl<V> Debug for dyn Task<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Task{{}}")
    }
}

pub trait Rebuilder<K, V> {
    type OutputTask: Task<V>;
    fn build(&self, key: K, current_value: V, task: &dyn Task<V>) -> Self::OutputTask;
}

pub trait Scheduler<K, V, VV> {
    // Says that the rebuilder must produce tasks that this scheduler can run.
    type RunTask: Task<VV>;
    fn schedule(&self, rebuilder: &dyn Rebuilder<K, V, OutputTask = Self::RunTask>, start: Vec<K>);
}
