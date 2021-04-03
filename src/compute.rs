use core::hash::Hash;
use std::collections::HashMap;
use std::sync::mpsc;
use crossbeam::channel;
use rayon::prelude::*;




/**
 * Interface for a task which can be performed in parallel with a group of
 * peers. Each task may require a subset of its peers to be executed.
 */
pub trait Compute: Sized {

    type Key;

    type Value;

    fn key(&self) -> Self::Key;

    fn peer_keys(&self) -> Vec<Self::Key>;

    fn run(&self, peers: Vec<Self>) -> Self::Value;
}




// ============================================================================
fn get_all<K, V>(map: &HashMap<K, V>, keys: Vec<K>) -> Option<Vec<V>>
where
    K: Hash + Eq,
    V: Clone
{
    keys.iter().map(|k| map.get(k).cloned()).collect()
}

fn into_mpsc_channel<A, T>(container: A) -> mpsc::Receiver<T>
where
    A: IntoIterator<Item = T>
{
    let (s, r) = mpsc::channel();

    for x in container {
        s.send(x).unwrap();
    }
    r
}

fn into_crossbeam_channel<A, T>(container: A) -> channel::Receiver<T>
where
    A: IntoIterator<Item = T>
{
    let (s, r) = channel::unbounded();

    for x in container {
        s.send(x).unwrap();
    }
    r
}




// ============================================================================
fn exec_with_mpsc_channel_internal<'a, C, K, V>(
    scope: &rayon::Scope<'a>,
    stage: mpsc::Receiver<(K, C)>) -> mpsc::Receiver<(K, V)>
where
    C: 'a + Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: 'a + Send + Sync + Clone + Hash + Eq,
    V: 'a + Send
{
    let (send, source) = mpsc::channel();
    let (sink, output) = mpsc::channel();

    let mut seen: HashMap<K, C> = HashMap::new();
    let mut hold = Vec::new();

    scope.spawn(|_| {
        source
        .into_iter()
        .par_bridge()
        .for_each_with(sink, |sink, (key, item, peers): (K, C, Vec<C>)| {
            sink.send((key.clone(), item.run(peers))).unwrap();
        });
    });

    for (key, item) in stage {
        seen.insert(key.clone(), item);
        hold.push(key);

        // see https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain_filter
        let mut i = 0;
        while i != hold.len() {
            let key = &hold[i];
            let held = &seen[key];
            if let Some(peers) = get_all(&seen, held.peer_keys()) {
                send.send((hold.remove(i), held.clone(), peers)).unwrap();
            } else {
                i += 1
            }
        }
    }

    assert!(hold.is_empty(), "there were {} unevaluated computes", hold.len());
    output
}




// ============================================================================
fn exec_with_crossbeam_channel_internal<'a, C, K, V>(
    scope: &rayon::Scope<'a>,
    stage: channel::Receiver<(K, C)>) -> channel::Receiver<(K, V)>
where
    C: 'a + Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: 'a + Send + Sync + Clone + Hash + Eq,
    V: 'a + Send
{
    let (send, source) = channel::unbounded();
    let (sink, output) = channel::unbounded();

    let mut seen: HashMap<K, C> = HashMap::new();
    let mut hold = Vec::new();

    scope.spawn(|_| {
        source
        .into_iter()
        .par_bridge()
        .for_each_with(sink, |sink, (key, item, peers): (K, C, Vec<C>)| {
            sink.send((key.clone(), item.run(peers))).unwrap();
        });
    });

    for (key, item) in stage {
        seen.insert(key.clone(), item);
        hold.push(key);

        // see https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain_filter
        let mut i = 0;
        while i != hold.len() {
            let key = &hold[i];
            let held = &seen[key];
            if let Some(peers) = get_all(&seen, held.peer_keys()) {
                send.send((hold.remove(i), held.clone(), peers)).unwrap();
            } else {
                i += 1
            }
        }
    }

    assert!(hold.is_empty(), "there were {} unevaluated computes", hold.len());
    output
}




/**
 * Execute a group of compute tasks using a conventional serial iterator.
 */
pub fn exec_with_serial_iterator<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: IntoIterator<Item = C>,
    C: Compute<Key = K, Value = V> + Clone,
    K: Hash + Eq + Clone,
{
    let stage: HashMap<_, _> = stage.into_iter().map(|c| (c.key(), c)).collect();
    let stage: HashMap<_, _> = stage.iter().map(|(k, compute)| {
        (k.clone(), compute.run(get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect();
    stage.into_iter()
}




/**
 * Execute a group of compute tasks using a parallel iterator from rayon.
 */
pub fn exec_with_parallel_iterator<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: IntoIterator<Item = C>,
    C: Sync + Compute<Key = K, Value = V> + Clone,
    K: Send + Sync + Hash + Eq + Clone,
    V: Send,
{
    let stage: HashMap<_, _> = stage.into_iter().map(|c| (c.key(), c)).collect();
    let stage: HashMap<_, _> = stage.par_iter().map(|(k, compute)| {
        (k.clone(), compute.run(get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect();
    stage.into_iter()
}




/**
 * Execute a group of compute tasks using parallel iterators from rayon and mpsc
 * channels. The tasks execute in the rayon global thread pool. If the task
 * group requires multiple stages to be fully evaluated, this function may be
 * invoked repeatedly with the output channel from the previous invocation,
 * potentially minimizing the number of idle threads.
 */
pub fn exec_with_mpsc_channel<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: Send + IntoIterator<Item = C>,
    C: Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: Send + Sync + Clone + Hash + Eq,
    V: Send,
{
    rayon::scope(|scope| {
        let stage = into_mpsc_channel(stage.into_iter().map(|c| (c.key(), c)));
        exec_with_mpsc_channel_internal(scope, stage).into_iter()
    })
}




/**
 * Execute a group of compute tasks using parallel iterators from rayon and
 * crossbeam channels. The tasks execute in the rayon global thread pool. If the
 * task group requires multiple stages to be fully evaluated, this function may
 * be invoked repeatedly with the output channel from the previous invocation,
 * potentially minimizing the number of idle threads.
 */
pub fn exec_with_crossbeam_channel<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: Send + IntoIterator<Item = C>,
    C: Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: Send + Sync + Clone + Hash + Eq,
    V: Send,
{
    rayon::scope(|scope| {
        let stage = into_crossbeam_channel(stage.into_iter().map(|c| (c.key(), c)));
        exec_with_crossbeam_channel_internal(scope, stage).into_iter()
    })
}
