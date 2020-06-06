use crossbeam::{
    deque::{Injector, Steal},
    scope,
};
use scopeguard::{defer, defer_on_unwind};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{sync_channel, Receiver},
};

pub trait CommandPoolTask: Send {
    type Result: Send;
    fn run(&self) -> Self::Result;
}

enum QueueTask<T: CommandPoolTask> {
    Stop,
    Task(T),
}

impl<T> std::fmt::Debug for QueueTask<T>
where
    T: CommandPoolTask,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            QueueTask::Stop => write!(f, "QueueTask::Stop"),
            QueueTask::Task(_) => write!(f, "QueueTask::Task"),
        }
    }
}

pub struct CommandPool<T: CommandPoolTask> {
    capacity: usize,
    job_queue: Injector<QueueTask<T>>,
    running_jobs: AtomicUsize,
}

pub struct Scope<'a, T: CommandPoolTask> {
    command_pool: &'a CommandPool<T>,
    pub rx: Receiver<T::Result>,
}

// Prevents users from enqueueing tasks outside run().
impl<'a, T> Scope<'a, T>
where
    T: CommandPoolTask,
{
    pub fn enqueue(&self, job: T) {
        self.command_pool.enqueue(job);
    }

    pub fn has_capacity(&self) -> bool {
        self.command_pool.has_capacity()
    }
}

impl<T> CommandPool<T>
where
    T: CommandPoolTask,
{
    pub fn new() -> Self {
        Self::with_capacity(num_cpus::get())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        CommandPool {
            capacity,
            job_queue: crossbeam::deque::Injector::new(),
            running_jobs: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn run<F, R>(&self, main_thread: F) -> Result<R, Box<dyn core::any::Any + 'static + Send>>
    where
        F: FnOnce(Scope<T>) -> R,
    {
        defer! {self.assert_no_running_jobs();}
        let (tx, rx) = sync_channel(self.capacity);

        // TODO: Any thread panics should also shut down all threads.
        scope(|s| {
            for _ in 0..self.capacity {
                let tx = tx.clone();
                // handles will be collected by the scope.
                s.spawn(move |_| {
                    defer_on_unwind! {
                        for _ in 0..self.capacity {
                            self.job_queue.push(QueueTask::Stop);
                        }
                    }

                    loop {
                        if let Steal::Success(task) = self.job_queue.steal() {
                            match task {
                                QueueTask::Stop => break,
                                QueueTask::Task(task) => {
                                    self.running_jobs.fetch_add(1, Ordering::SeqCst);
                                    defer! {self.running_jobs.fetch_sub(1, Ordering::SeqCst);}
                                    let result = task.run();
                                    tx.send(result)
                                        .expect("receiving side must not have panicked");
                                }
                            }
                        }
                    }
                });
            }

            // Drop rx so when threads exit, tx will close.
            drop(tx);

            {
                // shut down the threads even if the main thread panics.
                defer!(for _ in 0..self.capacity {
                    self.job_queue.push(QueueTask::Stop);
                });
                main_thread(Scope {
                    command_pool: &self,
                    rx,
                })
            }
        })
    }

    fn has_capacity(&self) -> bool {
        self.running_jobs.load(Ordering::Relaxed) < self.capacity
    }

    fn enqueue(&self, job: T) {
        self.job_queue.push(QueueTask::Task(job));
    }

    #[cfg(any(debug, test))]
    fn assert_no_running_jobs(&self) {
        assert_eq!(self.running_jobs.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(any(debug, test)))]
    fn assert_no_running_jobs(&self) {}
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        sync::{atomic::AtomicU8, Arc},
        thread::sleep,
        time::Duration,
    };

    struct PanickingTask {
        should_panic: bool,
        counter: Option<Arc<AtomicU8>>,
    }

    impl CommandPoolTask for PanickingTask {
        type Result = ();

        fn run(&self) {
            if self.should_panic {
                panic!("OOPS SOMETHING WENT WRONG!");
            } else {
                sleep(Duration::from_millis(10));
                self.counter
                    .as_ref()
                    .map(|c| c.fetch_add(1, Ordering::SeqCst));
            }
        }
    }

    macro_rules! panicking_task {
        ($should_panic:literal) => {
            PanickingTask {
                should_panic: $should_panic,
                counter: None,
            }
        };
        ($should_panic:literal, $counter:expr) => {
            PanickingTask {
                should_panic: $should_panic,
                counter: Some($counter),
            }
        };
    }

    struct AddingTask {
        counter: Arc<AtomicUsize>,
    }

    impl CommandPoolTask for AddingTask {
        type Result = usize;

        fn run(&self) -> Self::Result {
            sleep(Duration::from_millis(10));
            self.counter.fetch_add(1, Ordering::SeqCst)
        }
    }

    macro_rules! adding_task {
        ($counter:expr) => {
            AddingTask { counter: $counter }
        };
    }

    #[test]
    fn test_thread_panic() {
        // test that thread panic returns but _after_ finishing running tasks.
        // all threads should exit.
        let counter = Arc::new(AtomicU8::default());

        let pool = CommandPool::new();
        pool.run(|s| {
            s.enqueue(panicking_task!(false, counter.clone()));
            s.enqueue(panicking_task!(true));
            // Can't say anything about this task. If it is enqueued before the Stops it will run,
            // otherwise it won't.
            s.enqueue(panicking_task!(false, counter.clone()));
        })
        .expect_err("Expected pool to panic");
        assert!(counter.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn test_main_panic() {
        // all threads exit when main thread fails.
        let counter = Arc::new(AtomicU8::default());

        std::panic::catch_unwind(|| {
            let pool = CommandPool::new();
            let _ = pool.run(|s| {
                s.enqueue(panicking_task!(false, counter.clone()));
                s.enqueue(panicking_task!(false, counter.clone()));
                panic!("OOPS main thread failed");
            });
        })
        .expect_err("main thread must panic");

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_main_did_not_consume() {
        // test that main shutdown returns but _after_ finishing pending tasks.
        let counter = Arc::new(AtomicUsize::default());
        let pool = CommandPool::new();
        pool.run(|s| {
            for _i in 0..20 {
                s.enqueue(adding_task!(counter.clone()));
            }
        })
        .expect_err("pool to fail since main thread did not read results");
    }

    #[test]
    fn test_main_consume() {
        // test that main shutdown returns but _after_ finishing pending tasks.
        let counter = Arc::new(AtomicUsize::default());
        let pool = CommandPool::with_capacity(2);
        let upto = 20;
        pool.run(|s| {
            for _ in 0..upto {
                s.enqueue(adding_task!(counter.clone()));
                // Cheat a bit to get the desired effect.
                pool.job_queue.push(QueueTask::Stop);
            }

            // since each thread drops its sender, this will fail with Err eventually.
            while let Ok(_) = s.rx.recv() {}
        })
        .expect("pool succeeded");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert!(!pool.job_queue.is_empty());
    }

    #[test]
    fn test_enqueue_more() {
        // test that enqueueing more than capacity tasks does not block.
        let counter = Arc::new(AtomicUsize::default());
        let pool = CommandPool::with_capacity(2);
        let upto = 20;
        pool.run(|s| {
            for _ in 0..upto {
                s.enqueue(adding_task!(counter.clone()));
            }

            // Remember. Nothing guarantees that job results are received in the same order as they finish, so don't try to match i to a monotonically increasing sequence.
            // since each thread drops its sender, this will fail with Err eventually.
            let mut received = Vec::with_capacity(upto);
            while let Ok(i) = s.rx.recv() {
                received.push(i);
                if received.len() == upto {
                    break;
                }
            }
            received.sort();
            assert_eq!(received.into_iter().sum::<usize>(), 190);
        })
        .expect("pool succeeded");
        assert_eq!(counter.load(Ordering::SeqCst), upto);
        assert!(pool.job_queue.is_empty());
    }

    #[test]
    fn test_capacity_stops_are_consumed() {
        // test that enqueueing more than capacity tasks does not block.
        let counter = Arc::new(AtomicUsize::default());
        let pool = CommandPool::with_capacity(2);
        let upto = 20;
        pool.run(|s| {
            for _ in 0..upto {
                s.enqueue(adding_task!(counter.clone()));
            }

            // Cheat a bit to get the desired effect.
            for _ in 0..2 {
                pool.job_queue.push(QueueTask::Stop);
            }

            // since each thread drops its sender, this will fail with Err eventually.
            let mut expected = 0;
            while let Ok(_) = s.rx.recv() {
                expected += 1;
            }
            assert_eq!(expected, upto);
        })
        .expect("pool succeeded");

        assert_eq!(counter.load(Ordering::SeqCst), upto);

        // The pool itself will queue Stops when it exits. Consume those.
        let mut stops_left = 0;
        while let Steal::Success(task) = pool.job_queue.steal() {
            assert!(matches!(task, QueueTask::Stop));
            stops_left += 1;
        }
        assert_eq!(stops_left, 2);
        assert!(pool.job_queue.is_empty());
    }
}
