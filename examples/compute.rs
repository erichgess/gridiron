#![feature(drain_filter)]




use core::hash::Hash;
use std::collections::HashMap;
use std::sync::mpsc;
use rayon::prelude::*;




/**
 * Interface for a task which can be performed in parallel with a group of
 * peers. Each task may require a subset of its peers to be evaluated.
 */
trait Compute: Sized {

    type Key;

    type Value;

    fn key(&self) -> Self::Key;

    fn peer_keys(&self) -> Vec<Self::Key>;

    fn run(&self, peers: &[&Self]) -> Self::Value;
}




// ============================================================================
fn get_all<'a, K, V>(map: &'a HashMap<K, V>, keys: Vec<K>) -> Option<Vec<&'a V>>
where
    K: Hash + Eq,
{
    keys.iter().map(|k| map.get(k)).collect()
}

fn get_all_cloned<K, V>(map: &HashMap<K, V>, keys: Vec<K>) -> Option<Vec<V>>
where
    K: Hash + Eq,
    V: Clone
{
    keys.iter().map(|k| map.get(k).cloned()).collect()
}

fn into_channel<A, T>(container: A) -> mpsc::Receiver<T>
where
    A: IntoIterator<Item = T>
{
    let (s, r) = mpsc::channel();

    for x in container {
        s.send(x).unwrap();
    }
    r
}




// ============================================================================
fn execute_one_stage_par_channel_internal<'a, C, K, V>(
    scope: &rayon::ScopeFifo<'a>,
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

    scope.spawn_fifo(|_| {
        source
        .into_iter()
        .par_bridge()
        .for_each_with(sink, |sink, (key, item, peers): (K, C, Vec<C>)| {
            let peers: Vec<_> = peers.iter().collect();
            sink.send((key.clone(), item.run(&peers))).unwrap();
        });
    });

    for (key, item) in stage {
        seen.insert(key.clone(), item);
        hold.push(key);
        hold.drain_filter(|key| {
            let held = seen.get(key).unwrap();
            if let Some(peers) = get_all_cloned(&seen, held.peer_keys()) {
                send.send((key.clone(), held.clone(), peers)).unwrap();
                true
            } else {
                false
            }
        });
    }

    assert!(hold.is_empty(), "there were {} unevaluated computes", hold.len());
    output
}




// ============================================================================
fn execute_one_stage_channel<C, K, V>(stage: HashMap<K, C>) -> HashMap<K, V>
where
    C: Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: Send + Sync + Clone + Hash + Eq + std::fmt::Debug,
    V: Send
{
    rayon::scope_fifo(|scope| {
        execute_one_stage_par_channel_internal(scope, into_channel(stage)).into_iter().collect()
    })
}

fn execute_two_stage_channel<C, D, K, V>(stage: HashMap<K, C>) -> HashMap<K, V>
where
    C: Send + Sync + Clone + Compute<Key = K, Value = D>,
    D: Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: Send + Sync + Clone + Hash + Eq + std::fmt::Debug,
    V: Send
{
    rayon::scope_fifo(|scope| {
        let stage_b = execute_one_stage_par_channel_internal(scope, into_channel(stage));
        let stage_c = execute_one_stage_par_channel_internal(scope, stage_b);
        stage_c.iter().collect()
    })
}

// ============================================================================
fn execute_one_stage_par<C, K, V>(stage: HashMap<K, C>) -> HashMap<K, V>
where
    C: Compute<Key = K, Value = V> + Sync,
    K: Sync + Send + Hash + Eq + Clone,
    V: Send
{
    stage.par_iter().map(|(k, compute)| {
        (k.clone(), compute.run(&get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect()
}

// ============================================================================
fn execute_one_stage_ser<C, K, V>(stage: HashMap<K, C>) -> HashMap<K, V>
where
    C: Compute<Key = K, Value = V>,
    K: Hash + Eq + Clone,
{
    stage.iter().map(|(k, compute)| {
        (k.clone(), compute.run(&get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect()
}




// ============================================================================
#[derive(Clone)]
struct StringConvolve {
    index: usize,
    group_size: usize,
}




// ============================================================================
impl Compute for StringConvolve {

    type Key = usize;
    type Value = String;

    fn key(&self) -> Self::Key {
        self.index
    }

    fn peer_keys(&self) -> Vec<Self::Key> {
        vec![(self.index + self.group_size - 1) % self.group_size, (self.index + 1) % self.group_size]
    }

    fn run(&self, peers: &[&Self]) -> Self::Value {
        format!("{} {} {}", peers[0].index, self.index, peers[1].index)
    }
}




// ============================================================================
fn main() {

    let group_size = 10;
    let stage_a = (0..group_size).map(|index| StringConvolve { index, group_size });

    println!("\n--------------------------------------------");
    {
        let stage_a: HashMap<_, _> = stage_a.clone().map(|c| (c.key(), c)).collect();
        let stage_b = execute_one_stage_channel(stage_a);

        for (key, result) in stage_b {
            println!("{} -> {}", key, result);        
        }
    }
    println!("\n--------------------------------------------");

    {
        let stage_a: HashMap<_, _> = stage_a.clone().map(|c| (c.key(), c)).collect();
        let stage_b = execute_one_stage_ser(stage_a);

        for (key, result) in stage_b {
            println!("{} -> {}", key, result);        
        }
    }
    println!("\n--------------------------------------------");

    {
        let stage_a: HashMap<_, _> = stage_a.clone().map(|c| (c.key(), c)).collect();
        let stage_b = execute_one_stage_par(stage_a);

        for (key, result) in stage_b {
            println!("{} -> {}", key, result);        
        }
    }
}


































// fn execute_two_stage<C, D, K, V>(stage_a: HashMap<K, C>) -> HashMap<K, V>
// where
//     C: Compute<Key = K, Value = D>,
//     D: Compute<Key = K, Value = V>,
//     K: Hash + Eq + Clone,
// {
//     let stage_b = execute_one_stage(stage_a);
//     let stage_c = execute_one_stage(stage_b);
//     stage_c
// }




// fn execute_two_stage_par<C, D, K, V>(stage_a: HashMap<K, C>) -> HashMap<K, V>
// where
//     C: Compute<Key = K, Value = D> + Sync,
//     D: Compute<Key = K, Value = V> + Sync + Send,
//     K: Sync + Send + Hash + Eq + Clone,
//     V: Send
// {
//     let stage_b = execute_one_stage_par(stage_a);
//     let stage_c = execute_one_stage_par(stage_b);
//     stage_c
// }
