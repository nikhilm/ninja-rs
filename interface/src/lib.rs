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
    fn build(&self, key: K, current_value: V, task: &dyn Task<V>) -> V;
}

pub trait Scheduler<K, V> {
    fn schedule(&self, rebuilder: &dyn Rebuilder<K, V>, start: Vec<K>);
}
