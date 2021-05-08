use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use crossbeam_channel::{Receiver, Sender};
use log::{debug, error};

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
    ordered_inbound_sink: Sender<Vec<u8>>,
}

impl Orderer {
    pub fn new(
        initial_iteration: usize,
        inbound_recv: Receiver<Envelope>,
        tcp_out_sink: Sender<(usize, Iteration, Vec<u8>)>,
    ) -> (Orderer, Receiver<Vec<u8>>, Sender<(usize, Vec<u8>)>) {
        let cur_iteration = Arc::new(AtomicUsize::new(initial_iteration));

        let buffer = Arc::new(Mutex::new(HashMap::new()));
        let (ordered_inbound_sink, ordered_inbound_src) = crossbeam_channel::unbounded();
        {
            let aic = ordered_inbound_sink.clone();
            let c_iter = Arc::clone(&cur_iteration);
            let buffer = Arc::clone(&buffer);

            std::thread::spawn(move || {
                for env in inbound_recv {
                    let c_iter = c_iter.load(Ordering::SeqCst);
                    if env.iteration < c_iter {
                        error!("Received message with iteration number that precedes current iteration number");
                        error!("Dropping message");
                    } else if env.iteration == c_iter {
                        aic.send(env.data).unwrap();
                    } else {
                        debug!(
                            "Message received for a future iteration ({}), bufferering",
                            env.iteration
                        );
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

        let (outbound_sink, outbound_src) = crossbeam_channel::unbounded();
        {
            let tcp_out_sink = tcp_out_sink.clone();
            let cur_iteration = Arc::clone(&cur_iteration);
            std::thread::spawn(move || {
                for (dest, data) in outbound_src {
                    let iteration = cur_iteration.load(Ordering::SeqCst);
                    tcp_out_sink.send((dest, iteration, data)).unwrap();
                }
            });
        }

        (
            Orderer {
                cur_iteration,
                buffer,
                ordered_inbound_sink,
            },
            ordered_inbound_src,
            outbound_sink,
        )
    }

    pub fn increment(&mut self) {
        self.cur_iteration.fetch_add(1, Ordering::SeqCst);
        let i = self.cur_iteration.load(Ordering::SeqCst);

        let mut buffer = self.buffer.lock().unwrap();
        match buffer.get_mut(&self.cur_iteration.load(Ordering::SeqCst)) {
            Some(msgs) => {
                debug!(
                    "Flushing {} messages which were buffered for iteration: {}",
                    msgs.len(),
                    i
                );
                while let Some(msg) = msgs.pop() {
                    self.ordered_inbound_sink.send(msg).unwrap();
                }
            }
            None => (),
        }
    }
}
