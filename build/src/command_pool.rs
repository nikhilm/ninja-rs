use crossbeam::{
    deque::{Injector, Steal},
    scope,
};
use num_cpus;
use scopeguard::{defer, defer_on_unwind};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
};

pub trait CommandPoolTask: Send {
    type Result: Send;
    fn run(&self) -> Self::Result;
}

enum QueueTask<T: CommandPoolTask> {
    Stop,
    Task(T),
}

pub struct CommandPool<T: CommandPoolTask> {
    capacity: usize,
    job_queue: crossbeam::deque::Injector<QueueTask<T>>,
    running_jobs: std::sync::atomic::AtomicUsize,
}

impl<T> CommandPool<T>
where
    T: CommandPoolTask,
{
    pub fn new() -> Self {
        let capacity = num_cpus::get();
        CommandPool {
            capacity,
            job_queue: crossbeam::deque::Injector::new(),
            running_jobs: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn run<F, R>(&self, main_thread: F) -> R
    where
        F: FnOnce(Receiver<T::Result>) -> R,
    {
        let (tx, rx) = std::sync::mpsc::sync_channel(self.capacity);

        // TODO: Any thread panics should also shut down all threads.
        scope(|s| {
            for _ in 0..self.capacity {
                // handles will be collected by the scope.
                s.spawn(|_| {
                    defer_on_unwind!(for _ in 0..self.capacity {
                        self.job_queue.push(QueueTask::Stop);
                    });

                    loop {
                        if let Steal::Success(task) = self.job_queue.steal() {
                            match task {
                                QueueTask::Stop => break,
                                QueueTask::Task(task) => {
                                    self.running_jobs.fetch_add(1, Ordering::SeqCst);
                                    defer! {self.running_jobs.fetch_sub(1, Ordering::SeqCst);}
                                    let result = task.run();
                                    tx.send(result).unwrap();
                                }
                            }
                        }
                    }
                });
            }

            {
                // Regardless of what happens in the main callable, when that is done, shut down
                // the pool.
                defer!(for _ in 0..self.capacity {
                    self.job_queue.push(QueueTask::Stop);
                });
                return main_thread(rx);
            }
        })
        .expect("no crossbeam errors")
    }

    pub fn has_capacity(&self) -> bool {
        self.running_jobs.load(Ordering::Relaxed) < self.capacity
    }

    pub fn enqueue(&self, job: T) {
        self.job_queue.push(QueueTask::Task(job));
    }
}
