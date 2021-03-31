use core::hash::Hash;
use std::sync::mpsc;
use std::collections::HashMap;
use rayon::prelude::*;




/**
 * A custom coroutine proof-of-concept
 */
trait Compute: Sized {
    type Key: Send + Hash + Eq;
    type Value;

    fn key(&self) -> Self::Key;
    fn upstream(&self) -> Vec<Self::Key>;
    fn advance(&mut self, upstream: Vec<(Self::Key, Self)>);
}




/**
 * Sample executor for the compute coroutine.
 */
fn _execute<C: Compute + Send + Clone>(computes: Vec<C>) -> Vec<(C::Key, C::Value)> {


    let (sink1, inbox1) = mpsc::channel();
    let (sink2, inbox2) = mpsc::channel();
    let (sink3, _nbox3) = mpsc::channel();


    computes
        .into_par_iter()
        .for_each_with(sink1, |sink, mut compute|
    {
        compute.advance(Vec::new());
        sink.send(compute).unwrap();
    });


    rayon::scope(move |_| {
        let mut received = HashMap::new();
        for item in inbox1 {
            received.insert(item.key(), item.clone());
            if let Some(upstream) = item
                .upstream()
                .into_iter()
                .map(|key| received
                        .get(&key)
                        .cloned()
                        .map(|compute| (key, compute)))
                .collect::<Option<Vec<_>>>()
            {
                sink2.send((upstream, item)).unwrap();
            }
        }
    });


    rayon::scope(move |_| {
        inbox2
            .into_iter()
            .par_bridge()
            .for_each_with(sink3, |sink, (upstream, mut item)|
        {
            item.advance(upstream);
            sink.send(item).unwrap();
        });
    });


    panic!()
}




// ============================================================================
fn main() {
    let (s, r) = mpsc::channel();

    for i in 0..200 {
        s.send(i).unwrap()
    }
    drop(s);

    rayon::scope(|_| {
        r.into_iter()
         .par_bridge()
         .for_each(|item| {
            println!("{}/{}: {}", rayon::current_thread_index().unwrap(), rayon::current_num_threads(), item);
        })
    });
}
