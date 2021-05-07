use std::time::Duration;

use log::info;

/// Provides a mechanism for managing attempting to complete an operation
/// and retrying the operation, which a backoff, if it fails.
///
/// This iterator provides an infinite stream of back off durations, where
/// the duration increases an exponential factor up to some maximum delay.
/// Upon reaching the maximum delay, that value will be returned from then
/// on.
///
/// Higher order functions are provided which will manage the attempt to
/// execute a function and the retry and sleep logic.  These functions use
/// [std::thread::sleep] for the delay; so, in its current design, do NOT
/// use this with asynchronous code (e.g. `tokio`).
pub struct ExponentialBackoff {
    curr: Duration,
    max: Duration,
    factor: u32,
}

impl ExponentialBackoff {
    pub fn new(start: Duration, max: Duration, factor: u32) -> ExponentialBackoff {
        ExponentialBackoff {
            curr: start,
            max,
            factor,
        }
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let new_next = self.curr * self.factor;

        self.curr = if new_next > self.max {
            self.max
        } else {
            new_next
        };

        Some(self.curr)
    }
}

impl ExponentialBackoff {
    /// Retry the given function until it returns `Ok`. On an error, execute
    /// the `on_err` closure; this allows you to provide additional logic, like
    /// logging, on the error event which would otherwise be hidden by this
    /// function.
    ///
    /// Uses [std::thread::sleep] for the delay; so, in its current design, do NOT
    /// use this with asynchronous code (e.g. `tokio`).
    pub fn retry_forever<F, H, T, E>(&mut self, mut f: F, mut on_err: H) -> Option<T>
    where
        F: FnMut() -> Result<T, E>,
        H: FnMut(E),
    {
        for delay in self {
            match f() {
                Ok(t) => return Some(t),
                Err(e) => {
                    on_err(e);
                    info!("Retrying in {}ms...", delay.as_millis());
                    std::thread::sleep(delay);
                }
            }
        }

        None
    }

    /// Retry the given function until it returns `Ok` or `max_attempts` have been made.
    ///
    /// Uses [std::thread::sleep] for the delay; so, in its current design, do NOT
    /// use this with asynchronous code (e.g. `tokio`).
    pub fn retry_upto<F, T, E>(&mut self, max_attempts: usize, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
    {
        let mut enumerate = self.enumerate();
        loop {
            let (i, delay) = enumerate.next().unwrap();
            match f() {
                Ok(t) => return Ok(t),
                Err(e) => {
                    if i >= max_attempts {
                        return Err(e);
                    }
                    std::thread::sleep(delay)
                }
            }
        }
    }

    /// Retry the given function until it returns `Ok` or the `do_retry` predicate
    /// returns `false`.
    ///
    /// Uses [std::thread::sleep] for the delay; so, in its current design, do NOT
    /// use this with asynchronous code (e.g. `tokio`).
    pub fn retry_while<F, P, T, E>(&mut self, do_retry: P, f: F) -> Result<T, E>
    where
        F: Fn() -> Result<T, E>,
        P: Fn(usize, E) -> bool,
        E: Copy,
    {
        let mut enumerate = self.enumerate();
        loop {
            let (i, delay) = enumerate.next().unwrap();
            match f() {
                Ok(t) => return Ok(t),
                Err(e) => {
                    if do_retry(i, e) {
                        std::thread::sleep(delay);
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }
}
