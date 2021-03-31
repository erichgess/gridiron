use core::hash::Hash;
use std::collections::HashMap;




/**
 * A custom coroutine proof-of-concept
 */
trait Compute {

    /// Associated type to uniquely identify this compute task to its peers.
    type Key;

    /// Type of the result eventually yielded from this compute task.
    type Value;

    /// Return this compute task's key.
    fn key(&self) -> Self::Key;

    /// Return the upstream keys required for this task to proceed to the next
    /// stage.
    fn upstream(&self) -> Vec<Self::Key>;

    /// Upload the peer values associated with the upstream keys. Note that
    /// the upstream keys can change after a call to `advance`.
    fn upload(&mut self, upstream: Vec<Self>) where Self: Sized;

    /// Advance this compute task's internal state by performing an expensive
    /// calculation. The result of `upstream` can change each time this method
    /// is invoked.
    fn advance(self) -> Self;

    /// Return this compute tasks's result, if it's ready.
    fn poll(&mut self) -> Option<Self::Value>;
}




#[derive(Clone)]

/**
 * An example compute object. It's a crude state machine.
 */
struct MyCompute {
    key: i64,
    value: f64,
    upstream: Option<Vec<f64>>,
    result: Option<f64>,
}




// ============================================================================
impl MyCompute {
    fn new(key: i64, value: f64) -> Self {
        Self {
            key,
            value,
            upstream: None,
            result: None,
        }
    }
}




// ============================================================================
impl Compute for MyCompute {

    type Key = i64;
    type Value = f64;

    fn key(&self) -> Self::Key {
        self.key
    }

    fn upstream(&self) -> Vec<Self::Key> {
        vec![(self.key + 100 - 1) % 100, (self.key + 1) % 100]
    }

    fn upload(&mut self, upstream: Vec<Self>) {
        self.upstream = Some(upstream.into_iter().map(|peer| peer.value).collect())
    }

    fn advance(mut self) -> Self {
        if let Some(upstream) = self.upstream.take() {
            self.result = Some((upstream[0] + self.value + upstream[1]) / 3.0)
        }
        self
    }

    fn poll(&mut self) -> Option<Self::Value> {
        self.result
    }
}




/**
 * Drive a collection of interdependent state machines. Currently this only
 * works for uniform state machines that complete after a single stage.
 */
fn execute<C, K>(computes: Vec<C>) -> Vec<(C::Key, C::Value)>
where
    C: Compute<Key = K> + Send + Clone,
    K: Hash + Eq + Send,
{
    let stage1: HashMap<_, _> = computes
        .into_iter()
        .map(|c| (c.key(), c.advance()))
        .collect();

    let stage2 = stage1.iter().map(|(_, compute)| {

        let upstream: Option<Vec<_>> = compute
            .upstream()
            .iter()
            .map(|key| stage1.get(key).cloned())
            .collect();

        let mut this_compute = compute.clone();

        this_compute.upload(upstream.expect("missing upstream keys!"));
        this_compute.advance()
    });

    stage2.map(|mut c| (c.key(), c.poll().unwrap())).collect()
}




// ============================================================================
fn main() {

    let computes: Vec<_> = (0..100).map(|i| MyCompute::new(i, i as f64)).collect();

    for (key, x) in execute(computes) {
        println!("{} -> {}", key, x);
    }
}
