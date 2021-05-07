use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use crossbeam_channel::{Receiver, Sender};
use log::error;

use super::tcp::Iteration;

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
    buffer: Arc<Mutex<HashMap<Iteration, Vec<Vec<u8>>>>>,
    sink: Sender<Vec<u8>>,
}

impl Orderer {
    pub fn new(initial_iteration: usize, chan: Receiver<Envelope>) -> (Orderer, Receiver<Vec<u8>>) {
        let cur_iteration = Arc::new(AtomicUsize::new(initial_iteration));

        let buffer = Arc::new(Mutex::new(HashMap::new()));
        let (arrival_in, arrival_out) = crossbeam_channel::unbounded();
        {
            let aic = arrival_in.clone();
            let c_iter = Arc::clone(&cur_iteration);
            let buffer = Arc::clone(&buffer);

            // TODO: note that this is a tight loop and will not be good to have if there are a lot of messages from teh future
            std::thread::spawn(move || {
                for env in chan {
                    let c_iter = c_iter.load(Ordering::SeqCst);
                    if env.iteration < c_iter {
                        error!("Received message with iteration number that precedes current iteration number");
                        error!("Dropping message");
                    } else if env.iteration == c_iter {
                        aic.send(env.data).unwrap();
                    } else {
                        //bic.send(env).unwrap();
                        buffer
                            .lock()
                            .unwrap()
                            .entry(env.iteration)
                            .or_insert(vec![])
                            .push(env.data)
                    }
                }
            });
        }

        (
            Orderer {
                cur_iteration,
                buffer,
                sink: arrival_in,
            },
            arrival_out,
        )
    }

    pub fn set_iteration(&mut self, i: usize) {
        self.cur_iteration.store(i, Ordering::SeqCst);

        let mut buffer = self.buffer.lock().unwrap();
        match buffer.get_mut(&self.cur_iteration.load(Ordering::SeqCst)) {
            Some(msgs) => {
                while let Some(msg) = msgs.pop() {
                    self.sink.send(msg).unwrap();
                }
            }
            None => (),
        }
    }
}
