use core::hash::Hash;
use std::collections::HashMap;
use rayon::prelude::*;




trait ComputeStage {

    type Key;

    type Value;

    fn key(&self) -> Self::Key;

    fn peer_keys(&self) -> &[Self::Key];

    fn run(&self, peers: Vec<&Self>) -> Self::Value;
}




fn get_all<'a, K, V>(map: &'a HashMap<K, V>, keys: &[K]) -> Option<Vec<&'a V>>
where
    K: Hash + Eq,
{
    keys.iter()
        .map(|k| map.get(k))
        .collect()
}




fn execute_one_stage_par<C, K, V>(stage: HashMap<K, C>) -> HashMap<K, V>
where
    C: Sync + ComputeStage<Key = K, Value = V>,
    K: Send + Sync + Hash + Eq + Clone,
    V: Send
{
    stage.par_iter().map(|(k, compute)| {
        (k.clone(), compute.run(get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect()
}




fn execute_two_stage_par<C, D, K, V>(stage_a: HashMap<K, C>) -> HashMap<K, V>
where
    C: Sync + ComputeStage<Key = K, Value = D>,
    D: Sync + ComputeStage<Key = K, Value = V> + Send,
    K: Send + Sync + Hash + Eq + Clone,
    V: Send
{
    let stage_b = execute_one_stage_par(stage_a);
    let stage_c = execute_one_stage_par(stage_b);
    stage_c
}




fn execute_one_stage<C, K, V>(stage: HashMap<K, C>) -> HashMap<K, V>
where
    C: ComputeStage<Key = K, Value = V>,
    K: Hash + Eq + Clone,
{
    stage.iter().map(|(k, compute)| {
        (k.clone(), compute.run(get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect()
}




fn execute_two_stage<C, D, K, V>(stage_a: HashMap<K, C>) -> HashMap<K, V>
where
    C: ComputeStage<Key = K, Value = D>,
    D: ComputeStage<Key = K, Value = V>,
    K: Hash + Eq + Clone,
{
    let stage_b = execute_one_stage(stage_a);
    let stage_c = execute_one_stage(stage_b);
    stage_c
}


















/**
 * A custom coroutine proof-of-concept
 */
trait Compute: Sized {

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
    fn upload(&mut self, upstream: Vec<Self>);

    /// Return this compute tasks's state, containing the result if it's ready.
    fn state(&mut self) -> Option<Self::Value>;

    /// Advance this compute task's internal state by performing an expensive
    /// calculation. The result of `upstream` can change each time this method
    /// is invoked.
    fn advance(&mut self);

    /// Convenience provided method to consume this compute, advance and return
    /// it.
    fn into_advance(mut self) -> Self {
        self.advance();
        self
    }
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

    fn advance(&mut self) {
        if let Some(upstream) = self.upstream.take() {
            self.result = Some((upstream[0] + self.value + upstream[1]) / 3.0)
        }
    }

    fn state(&mut self) -> Option<Self::Value> {
        self.result
    }
}




/**
 * Drive a collection of interdependent state machines.
 */
fn execute<C, K>(computes: Vec<C>) -> Vec<(C::Key, C::Value)>
where
    C: Compute<Key = K> + Send + Clone,
    K: Hash + Eq + Send,
{
    let num_tasks = computes.len();
    let mut results = Vec::new();

    let mut stage_a : HashMap<C::Key, C>;
    let mut stage_b = HashMap::new();

    stage_a = computes
        .into_iter()
        .map(|c| (c.key(), c.into_advance()))
        .collect();

    loop {
        let mut progress = false;

        for (_, compute) in &stage_a {

            let upstream: Option<Vec<_>> = compute
                .upstream()
                .iter()
                .map(|key| stage_a.get(key).cloned())
                .collect();

            let mut next = compute.clone();

            next.upload(upstream.expect("missing upstream keys!"));
            next.advance();

            if let Some(result) = next.state() {
                progress = true;
                results.push((next.key(), result))
            }
            stage_b.insert(next.key(), next);
        }

        if stage_b.len() == num_tasks {
            return results
        }
        if !progress {
            panic!("computation stage did not make progress")
        }
        std::mem::swap(&mut stage_a, &mut stage_b);
    }
}




// ============================================================================
fn main() {

    let computes: Vec<_> = (0..100).map(|i| MyCompute::new(i, i as f64)).collect();

    for (key, x) in execute(computes) {
        println!("{} -> {}", key, x);
    }
}
