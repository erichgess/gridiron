#![feature(drain_filter)]




use core::hash::Hash;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;
use crossbeam::channel;
use rayon::prelude::*;




/**
 * Interface for a task which can be performed in parallel with a group of
 * peers. Each task may require a subset of its peers to be executed.
 */
trait Compute: Sized {

    type Key;

    type Value;

    fn key(&self) -> Self::Key;

    fn peer_keys(&self) -> Vec<Self::Key>;

    fn run(&self, peers: &[&Self]) -> Self::Value;

    fn run_owned(&self, peers: Vec<Self>) -> Self::Value;
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
fn execute_one_stage_channel_internal<'a, C, K, V>(
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
            sink.send((key.clone(), item.run_owned(peers))).unwrap();
        });
    });

    for (key, item) in stage {
        seen.insert(key.clone(), item);
        hold.push(key);
        hold.drain_filter(|key| {
            let held = &seen[key];
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

fn execute_one_stage_crossbeam_channel_internal<'a, C, K, V>(
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
            sink.send((key.clone(), item.run_owned(peers))).unwrap();
        });
    });

    for (key, item) in stage {
        seen.insert(key.clone(), item);
        hold.push(key);
        hold.drain_filter(|key| {
            let held = &seen[key];
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
fn execute_one_stage_channel<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: Send + IntoIterator<Item = C>,
    C: Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: Send + Sync + Clone + Hash + Eq,
    V: Send,
{
    rayon::scope(|scope| {
        let stage = into_channel(stage.into_iter().map(|c| (c.key(), c)));
        execute_one_stage_channel_internal(scope, stage).into_iter()
    })
}

fn execute_one_stage_crossbeam_channel<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: Send + IntoIterator<Item = C>,
    C: Send + Sync + Clone + Compute<Key = K, Value = V>,
    K: Send + Sync + Clone + Hash + Eq,
    V: Send,
{
    rayon::scope(|scope| {
        let stage = into_crossbeam_channel(stage.into_iter().map(|c| (c.key(), c)));
        execute_one_stage_crossbeam_channel_internal(scope, stage).into_iter()
    })
}




// fn execute_two_stage_channel<I, C, D, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
// where
//     I: Send + IntoIterator<Item = (K, C)>,
//     C: Send + Sync + Clone + Compute<Key = K, Value = D>,
//     D: Send + Sync + Clone + Compute<Key = K, Value = V>,
//     K: Send + Sync + Clone + Hash + Eq,
//     V: Send
// {
//     rayon::scope_fifo(|scope| {
//         let stage_b = execute_one_stage_channel_internal(scope, into_channel(stage));
//         let stage_c = execute_one_stage_channel_internal(scope, stage_b);
//         stage_c.into_iter()
//     })
// }

// fn execute_n_stage_channel<I, C, K>(stage: I, num_stages: usize) -> impl Iterator<Item = (K, C)>
// where
//     I: Send + IntoIterator<Item = (K, C)>,
//     C: Send + Sync + Clone + Compute<Key = K, Value = C>,
//     K: Send + Sync + Clone + Hash + Eq,
// {
//     rayon::scope_fifo(|scope| {
//         let mut stage = into_channel(stage);
//         for _ in 0..num_stages {
//             stage = execute_one_stage_channel_internal(scope, stage);
//         }
//         stage.into_iter()
//     })
// }




// ============================================================================
fn execute_one_stage_ser<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: IntoIterator<Item = C>,
    C: Compute<Key = K, Value = V>,
    K: Hash + Eq + Clone,
{
    let stage: HashMap<_, _> = stage.into_iter().map(|c| (c.key(), c)).collect();
    let stage: HashMap<_, _> = stage.iter().map(|(k, compute)| {
        (k.clone(), compute.run(&get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect();
    stage.into_iter()
}




// ============================================================================
fn execute_one_stage_par<I, C, K, V>(stage: I) -> impl Iterator<Item = (K, V)>
where
    I: IntoIterator<Item = C>,
    C: Sync + Compute<Key = K, Value = V>,
    K: Send + Sync + Hash + Eq + Clone,
    V: Send,
{
    let stage: HashMap<_, _> = stage.into_iter().map(|c| (c.key(), c)).collect();
    let stage: HashMap<_, _> = stage.par_iter().map(|(k, compute)| {
        (k.clone(), compute.run(&get_all(&stage, compute.peer_keys()).expect("missing peers")))
    }).collect();
    stage.into_iter()
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

    fn run_owned(&self, peers: Vec<Self>) -> Self::Value {
        format!("{} {} {}", peers[0].index, self.index, peers[1].index)
    }
}




// ============================================================================
fn main() {

    let group_size = 10;
    let stage = (0..group_size).map(|index| StringConvolve { index, group_size });

    println!("\n--------------------------------------------");
    for (key, result) in execute_one_stage_channel(stage.clone()) {
        println!("{} -> {}", key, result);
    }

    println!("\n--------------------------------------------");
    for (key, result) in execute_one_stage_ser(stage.clone()) {
        println!("{} -> {}", key, result);
    }

    println!("\n--------------------------------------------");
    for (key, result) in execute_one_stage_par(stage.clone()) {
        println!("{} -> {}", key, result);
    }

    divergence_example();
    divergence_example();
    divergence_example();
    divergence_example();
}




// ============================================================================
fn divergence_example() {
    use gridiron::index_space::range2d;

    let num_blocks = (64, 64);
    let block_size = (64, 64);

    let blocks = range2d(0..num_blocks.0 as i64, 0..num_blocks.1 as i64);
    let peers: Vec<_> = blocks.into_iter().map(|ij| DivergenceStencil::new(block_size, num_blocks, ij)).collect();

    let start = std::time::Instant::now();

    for (_key, _result) in execute_one_stage_crossbeam_channel(peers) {
    }        
    println!("elapsed: {:.4}s", start.elapsed().as_secs_f64());
}




#[derive(Clone)]
struct DivergenceStencil {
    data: Arc<Vec<f64>>,
    shape: (usize, usize),
    block: (usize, usize),
    index: (i64, i64),
}

impl DivergenceStencil {
    fn new(shape: (usize, usize), block: (usize, usize), index: (i64, i64)) -> Self {
        Self {
            data: Arc::new(vec![0.0; shape.0 * shape.1]),
            shape,
            block,
            index,
        }
    }
}




// ============================================================================
impl Compute for DivergenceStencil {

    type Key = (i64, i64);

    type Value = Vec<f64>;

    fn key(&self) -> Self::Key {
        self.index
    }

    fn peer_keys(&self) -> Vec<Self::Key> {
        let (i, j) = self.index;
        let (l, m) = self.block;
        let (l, m) = (l as i64, m as i64);

        vec![
            ((i + l - 1) % l, (j + m) % m),
            ((i + l + 1) % l, (j + m) % m),
            ((i + l) % l, (j + m - 1) % m),
            ((i + l) % l, (j + m + 1) % m),
        ]
    }

    fn run(&self, peers: &[&Self]) -> Self::Value {
        let b00 = &self.data;
        let bxl = &peers[0].data;
        let bxr = &peers[1].data;
        let byl = &peers[2].data;
        let byr = &peers[3].data;

        let (l, m) = self.shape;

        let ind = |i, j| {
            i * m + j
        };
        let mut result = vec![0.0; self.data.len()];

        for i in 0..l {
            for j in 0..m {
                for _ in 0..5000 {
                    let cxl = if i == 0 { bxl[ind(l - 1, j)] } else { b00[ind(i, j)] };
                    let cxr = if i == l - 1 { bxr[ind(0, j)] } else { b00[ind(i, j)] };
                    let cyl = if j == 0 { byl[ind(i, m - 1)] } else { b00[ind(i, j)] };
                    let cyr = if j == m - 1 { byr[ind(i, 0)] } else { b00[ind(i, j)] };
                    result[ind(i, j)] = (cxr - cxl) + (cyr - cyl);
                }
            }
        }
        result
    }

    fn run_owned(&self, peers: Vec<Self>) -> Self::Value {
        let b00 = &self.data;
        let bxl = &peers[0].data;
        let bxr = &peers[1].data;
        let byl = &peers[2].data;
        let byr = &peers[3].data;

        let (l, m) = self.shape;

        let ind = |i, j| {
            i * m + j
        };
        let mut result = vec![0.0; self.data.len()];

        for i in 0..l {
            for j in 0..m {
                for _ in 0..5000 {
                    let cxl = if i == 0 { bxl[ind(l - 1, j)] } else { b00[ind(i, j)] };
                    let cxr = if i == l - 1 { bxr[ind(0, j)] } else { b00[ind(i, j)] };
                    let cyl = if j == 0 { byl[ind(i, m - 1)] } else { b00[ind(i, j)] };
                    let cyr = if j == m - 1 { byr[ind(i, 0)] } else { b00[ind(i, j)] };
                    result[ind(i, j)] = (cxr - cxl) + (cyr - cyl);
                }
            }
        }
        result
    }
}
