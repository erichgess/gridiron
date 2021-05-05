use std::time::Duration;

pub struct ExponentialBackoff {
    curr: Duration,
    max: Duration,
    factor: u32,
    iterations: usize,
    max_iterations: Option<usize>,
}

impl ExponentialBackoff {
    pub fn new(
        start: Duration,
        max: Duration,
        factor: u32,
        max_iterations: Option<usize>,
    ) -> ExponentialBackoff {
        ExponentialBackoff {
            curr: start,
            max,
            factor,
            iterations: 0,
            max_iterations,
        }
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self
            .max_iterations
            .map_or(false, |max| self.iterations >= max)
        {
            None
        } else {
            let new_next = self.curr * self.factor;

            self.curr = if new_next > self.max {
                self.max
            } else {
                new_next
            };

            self.iterations += 1;
            Some(self.curr)
        }
    }
}
