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
    current_worker_index: cell::Cell<usize>,
}

impl ThreadPool {
    /// Create a new thread pool with the given number of threads.
    ///
    pub fn new(num_threads: usize) -> Self {
        let workers = (0..num_threads)
            .map(|_| {
                let (sender, receiver): (mpsc::Sender<Job>, mpsc::Receiver<Job>) = mpsc::channel();
                let handle = thread::spawn(|| {
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
            current_worker_index: cell::Cell::new(0),
        }
    }

    /// Spawn a new job into the pool. Job submissions go cyclically to the
    /// workers: if worker `n` gets this job, then worker `(n + 1) %
    /// num_workers` gets the next one.
    ///
    pub fn spawn<F>(&self, job: F)
    where
        F: FnOnce() -> () + Send + 'static,
    {
        let mut index = self.current_worker_index.get();
        self.workers[index]
            .sender
            .as_ref()
            .unwrap()
            .send(Box::new(job))
            .unwrap();
        index += 1;
        index %= self.workers.len();
        self.current_worker_index.set(index);
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
