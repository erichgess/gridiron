use std::sync::mpsc;
use rayon::prelude::*;




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
