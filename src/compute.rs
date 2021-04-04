use core::hash::Hash;
use std::collections::HashMap;
use std::sync::mpsc;
use crossbeam_channel;
use rayon::prelude::*;




/**
 * Interface to enable parallel execution of directed acyclic graphs (DAGs) that
 * have a "layered" topology: meaning the DAG is decomposed into generations,
 * and edges only exist between adjacent generations. In other words, any task
 * in a group of tasks can be executed given a subset of its peers. Execution of
 * a task group is accompished by any of several executor functions, some
 * examples of which are given below.
 *
 * The compute task is likely to be cloned by its executor, so it is wise to put
 * any heavyweight data members under `Rc` or `Arc` (to enable multi-threaded
 * execution).
 */
pub trait Compute: Sized {

    /// The type of the key to uniquely identify this task within the task
    /// group. Executors will generally require this type to be `Clone + Hash
    /// + Eq`, and possibly `Send` or `Sync` if the executor is
    /// multi-threaded.
    type Key;

    /// The type of the value yielded when this task is run. This can be any
    /// type of single-stage evaluation. For a two-stage evaluation, this type
    /// must also be Compute. For an n-stage evaluation, this type must be
    /// Self.
    type Value;

    /// Return the keys of other members of the execution group which are
    /// required for this compute task to be evaluated.
    fn peer_keys(&self) -> Vec<Self::Key>;

    /// Return the key uniquely identify this task within the task group.
    fn key(&self) -> Self::Key;

    /// Run this task, given an owned vector of its peers. The executor is
    /// responsible for making sure the order of the peers is the same as the
    /// order of the `Vec` returned by the `peer_keys` method.
    fn run(&self, peers: Vec<Self>) -> Self::Value;
}




/**
 * Execute a group of compute tasks using a conventional serial iterator.
 */
pub fn exec_with_serial_iterator<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: IntoIterator<Item = C>,
    C: Clone + Compute<Key = K, Value = V>,
    K: Clone + Hash + Eq,
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
    C: Clone + Sync + Compute<Key = K, Value = V>,
    K: Clone + Send + Sync + Hash + Eq,
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
    C: Send + Clone + Compute<Key = K, Value = V>,
    K: Send + Clone + Hash + Eq,
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
    C: Send + Clone + Compute<Key = K, Value = V>,
    K: Send + Clone + Hash + Eq,
    V: Send,
{
    rayon::scope(|scope| {
        let stage = into_crossbeam_channel(stage.into_iter().map(|c| (c.key(), c)));
        exec_with_crossbeam_channel_internal(scope, stage).into_iter()
    })
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

fn into_crossbeam_channel<A, T>(container: A) -> crossbeam_channel::Receiver<T>
where
    A: IntoIterator<Item = T>
{
    let (s, r) = crossbeam_channel::unbounded();

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
    C: 'a + Send + Clone + Compute<Key = K, Value = V>,
    K: 'a + Send + Clone + Hash + Eq,
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
            let held = &seen[&hold[i]];
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
    stage: crossbeam_channel::Receiver<(K, C)>) -> crossbeam_channel::Receiver<(K, V)>
where
    C: 'a + Send + Clone + Compute<Key = K, Value = V>,
    K: 'a + Send + Clone + Hash + Eq,
    V: 'a + Send
{
    let (send, source) = crossbeam_channel::unbounded();
    let (sink, output) = crossbeam_channel::unbounded();

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
            let held = &seen[&hold[i]];
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
