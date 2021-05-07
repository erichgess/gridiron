use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crossbeam_channel::{Receiver, Sender};
use log::error;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Envelope {
    pub iteration: usize,
    pub data: Vec<u8>,
}

/// Orderer enforces the proper ordering on incoming messages and makes sure that
/// arriving messages which are from a future iteration are kept until the local
/// workers have reached that iteration.
pub struct Orderer {
    cur_iteration: Arc<AtomicUsize>,
}

impl Orderer {
    pub fn new(initial_iteration: usize, chan: Receiver<Envelope>) -> (Orderer, Receiver<Vec<u8>>) {
        let cur_iteration = Arc::new(AtomicUsize::new(initial_iteration));
        let (buffer_in, buffer_out): (Sender<Envelope>, Receiver<Envelope>) =
            crossbeam_channel::unbounded();

        {
            let bc = buffer_in.clone();
            std::thread::spawn(move || {
                for env in chan {
                    bc.send(env).unwrap();
                }
            });
        }

        let (arrival_in, arrival_out) = crossbeam_channel::unbounded();
        {
            let (bic, boc) = (buffer_in.clone(), buffer_out.clone());
            let aic = arrival_in.clone();
            let c_iter = Arc::clone(&cur_iteration);

            // TODO: note that this is a tight loop and will not be good to have if there are a lot of messages from teh future
            std::thread::spawn(move || {
                for env in boc {
                    let c_iter = c_iter.load(Ordering::SeqCst);
                    if env.iteration < c_iter {
                        error!("Received message with iteration number that precedes current iteration number");
                        error!("Dropping message");
                    } else if env.iteration == c_iter {
                        aic.send(env.data).unwrap();
                    } else {
                        bic.send(env).unwrap();
                    }
                }
            });
        }

        (Orderer { cur_iteration }, arrival_out)
    }

    /// Increments the iteration
    pub fn next_iteration(&mut self) {
        self.cur_iteration.fetch_add(1, Ordering::SeqCst);
    }

    pub fn set_iteration(&mut self, i: usize) {
        self.cur_iteration.store(i, Ordering::SeqCst)
    }
}
