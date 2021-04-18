use std::cell;
use std::sync::mpsc;
use std::thread;

type Job = Box<dyn FnOnce() -> () + Send + 'static>;

struct Worker {
    handle: Option<thread::JoinHandle<()>>,
    sender: Option<mpsc::Sender<Job>>,
}

/// A minimal thread pool implementation. No effort is made to schedule jobs
/// intelligently, it just goes round-robin. Jobs must be `'static`.
///
pub struct ThreadPool {
    workers: Vec<Worker>,
    current_worker_id: cell::Cell<usize>,
}

impl ThreadPool {
    /// Create a new thread pool with the given number of threads.
    ///
    pub fn new(num_threads: usize) -> Self {

        use core_affinity::{get_core_ids, set_for_current};

        let workers = get_core_ids().unwrap().into_iter().take(num_threads)
            .map(|core_id| {
                let (sender, receiver): (mpsc::Sender<Job>, mpsc::Receiver<Job>) = mpsc::channel();
                let handle = thread::spawn(move || {
                    set_for_current(core_id);
                    for job in receiver {
                        job()
                    }
                });
                Worker {
                    handle: Some(handle),
                    sender: Some(sender),
                }
            })
            .collect();

        ThreadPool {
            workers,
            current_worker_id: cell::Cell::new(0),
        }
    }

    /// Return the number of worker threads in the pool.
    /// 
    pub fn num_threads(&self) -> usize {
        self.workers.len()
    }

    /// Spawn a new job into the pool. Job submissions go cyclically to the
    /// workers: if worker `n` gets this job, then worker `(n + 1) %
    /// num_workers` gets the next one.
    ///
    pub fn spawn<F>(&self, job: F)
    where
        F: FnOnce() -> () + Send + 'static,
    {
        self.spawn_on(None, job)
    }

    /// Spawn a job onto the worker thread with the given index, if it is
    /// `Some`. The current worker index is not incremented. If the worker
    /// index is `None`, then the job is run on the current worker index,
    /// which is then incremented.
    ///
    pub fn spawn_on<F>(&self, worker_id: Option<usize>, job: F)
    where
        F: FnOnce() -> () + Send + 'static,
    {
        let worker_id = if let Some(worker_id) = worker_id {
            worker_id
        } else {
            let worker_id = self.current_worker_id.get();
            self.current_worker_id.set((worker_id + 1) % self.num_threads());
            worker_id
        };
        self.workers[worker_id]
            .sender
            .as_ref()
            .unwrap()
            .send(Box::new(job))
            .unwrap();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
