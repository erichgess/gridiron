use std::{iter::Take, time::Duration};

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

/// The Retry trait provides a protocol for handling retrying a function with a
/// [Result] type until either it succeeds or the [Iterator] completes.  This is
/// defined to be used on Iterators over [Duration] values, those values specfying
/// the amount of time to wait between each retry.
pub trait Retry {
    /// Retry the given function until it returns `Ok`. On an error, execute
    /// the `on_err` closure; this allows you to provide additional logic, like
    /// logging, on the error event which would otherwise be hidden by this
    /// function.
    ///
    /// Uses [std::thread::sleep] for the delay; so, in its current design, do NOT
    /// use this with asynchronous code (e.g. `tokio`).
    fn retry<F, H, T, E>(&mut self, mut f: F, on_err: H) -> Option<Result<T, E>>
    where
        F: FnMut() -> Result<T, E>,
        H: Fn(&E),
        Self: Iterator,
        Self::Item: Into<Duration>,
    {
        let mut last_err = None;
        for delay in self {
            match f() {
                Ok(t) => return Some(Ok(t)),
                Err(e) => {
                    on_err(&e);
                    last_err = Some(Err(e));
                    let delay: Duration = delay.into();
                    info!("Retrying in {}ms...", delay.as_millis());
                    std::thread::sleep(delay);
                }
            }
        }

        last_err
    }
}

impl Retry for ExponentialBackoff {}

impl Retry for Take<ExponentialBackoff> {}
