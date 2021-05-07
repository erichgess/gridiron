use std::{iter::Take, time::Duration};

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
    /// Retry the given function until it returns [Ok]. On an error, execute
    /// the `on_err` closure; this allows you to provide additional logic, like
    /// logging, on the error event which would otherwise be hidden by this
    /// function. If the [Iterator] finishes without a successful execution of
    /// `f` then the last [Err] is returned to the user.
    ///
    /// - `f` is the function which will be executed until an [Ok] is returned or the underlying
    /// iterator is empty
    /// - `sleep` is called after every attempt but the last and is used to handle
    /// the delay before the next attempt.  In addition, the error from the last attempt
    /// is provided so that you may log information.
    fn retry<F, S, T, E>(&mut self, mut f: F, sleep: S) -> Option<Result<T, E>>
    where
        F: FnMut() -> Result<T, E>,
        S: Fn(&E, Self::Item),
        Self: Iterator,
    {
        let mut last_err = None;
        let mut iter = self.peekable();
        loop {
            let is_last = iter.peek().is_none();

            match iter.next() {
                Some(delay) => match f() {
                    Ok(v) => return Some(Ok(v)),
                    Err(e) => {
                        if !is_last {
                            sleep(&e, delay.into());
                        }
                        last_err = Some(Err(e));
                    }
                },
                None => return last_err,
            }
        }
    }
}

impl Retry for ExponentialBackoff {}

impl Retry for Take<ExponentialBackoff> {}
