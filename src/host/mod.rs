use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use log::{debug, error, info, warn};

pub mod msg;
use crate::host::msg::Signal;
use crate::patch::Patch;

/// Constants
const RETRY_LIMIT: usize = 3;
const RETRY_DELAY_MS: u64 = 1000;
const POLL_TIMEOUT_MS: i64 = 10000;
const LINGER_PERIOD_MS: i32 = 10000;

/// The server will receive data pushed by peers.  It will Parse the event message
/// and act accordingly.  For a Data message, the received data will be stored in
/// memory and then an Ack sent back to the peer.
pub mod receiver {

    use super::*;

    pub fn receiver(port: u32, input_sender: Sender<Patch>, signal: Receiver<Signal>) {
        let context = zmq::Context::new();
        let responder = context.socket(zmq::REP).unwrap();
        responder.set_rcvtimeo(POLL_TIMEOUT_MS as i32).unwrap();
        let addr = format!("tcp://*:{}", port);
        assert!(responder.bind(&addr).is_ok());

        let mut msg = zmq::Message::new();
        loop {
            // Check for stop signal
            info!("Check for signals");
            match signal.try_recv() {
                Ok(Signal::Stop) => {
                    info!("Received shutdown signal");
                    break;
                }
                _ => (),
            }

            // Move this logic behind a channel so that I can run select? But then I am not guaranteed that
            // the Stop signal will be read if there are always messages in the Network channel
            info!("Listen for message");
            match responder.recv(&mut msg, 0) {
                Ok(()) => (),
                Err(_) => continue, // TODO: I don't like having the continue here because it makes it hard to see the cycles that have no exit
            }
            let req: msg::Request = rmp_serde::decode::from_slice(&msg).unwrap();
            info!("Message: {:?}", req);

            // Post message to a channel for processing and then send Ack
            match input_sender.send(req.data().clone()) {
                Ok(_) => debug!("Sent data to channel"),
                Err(msg) => error!("Failed to post to channel: {}", msg),
            }

            thread::sleep(Duration::from_millis(1000));

            info!("Sending Ack for {}", req.id());
            let response = msg::Response::new(msg::Status::Good(req.id()));
            let mpk = rmp_serde::encode::to_vec(&response).unwrap();
            responder.send(&mpk, 0).unwrap();
            info!("Ack Sent for {}", req.id());
        }
        info!("Stopping server thread");
    }
}

/**
This will connect to a peer and handle pushing new state data
 to the peer

 Some quick thoughts on this code:
 1. Find a design that makes sure that all paths will always go through the backoff process for retries.
 2. Find a design that will make it impossible to retry when the success state is achieved.  In the current code
 I have to remember to `break` after successfully receiving an `Ack` or I will just keep sending messages
*/
pub mod sender {
    use super::*;

    pub fn sender(addr: String, output_rcv: Receiver<Patch>) {
        // setup client to the peer at `port` when new data is ready
        // push that dato to the peer
        // Setup ZeroMQ
        info!("Connecting to {}...\n", addr);

        let context = zmq::Context::new();
        let mut requester = context.socket(zmq::REQ).unwrap();
        requester.set_linger(LINGER_PERIOD_MS).unwrap();
        debug!("Linger: {:?}", requester.get_linger());
        debug!("New Socket: {:?}", requester.get_identity().unwrap());
        assert!(requester.connect(&addr).is_ok());

        let mut request_nbr = 0;
        loop {
            request_nbr += 1;
            let data = match output_rcv.recv() {
                Ok(d) => d,
                Err(msg) => {
                    info!("Channel Disconnected: {}", msg);
                    break;
                }
            };

            info!("Sending Data ID {}...", request_nbr);
            let msg = msg::Request::new(request_nbr, &data);
            let mpk = rmp_serde::encode::to_vec(&msg).unwrap();

            // Push data to peer
            let mut attempts = 0;
            loop {
                attempts += 1;

                if attempts > RETRY_LIMIT {
                    error!(
                        "Exceeded max retry limit ({}). Dropping message",
                        RETRY_LIMIT
                    );
                    break;
                } else if attempts > 1 {
                    warn!("Wait {}ms then retry...", RETRY_DELAY_MS);
                    thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }

                match requester.send(&mpk, 0) {
                    Ok(_) => (),
                    Err(msg) => {
                        info!("Send Error: {}", msg);
                        continue;
                    }
                }

                // Wait for peer to Ack the message
                info!("Waiting for Ack for {}", request_nbr);
                match requester.poll(zmq::PollEvents::POLLIN, POLL_TIMEOUT_MS) {
                    Ok(i) => {
                        //
                        debug!("Polling #: {}", i);
                        if i > 0 {
                            let mut response = zmq::Message::new();
                            match requester.recv(&mut response, 0) {
                                Ok(_) => {
                                    let response: msg::Response =
                                        rmp_serde::decode::from_slice(&response).unwrap();
                                    match response.status() {
                                        msg::Status::Good(id) => {
                                            if id != request_nbr {
                                                warn!("Received Ack for wrong message.  Got {}, expected {}.", id, request_nbr);
                                            } else {
                                                info!("Received Ack for {}", request_nbr);
                                            }
                                        }
                                        msg::Status::Bad => {
                                            warn!("Received Bad from peer");
                                        }
                                    }
                                    break;
                                }
                                Err(msg) => {
                                    panic!("Receive Error: {}", msg);
                                }
                            }
                        } else {
                            info!("Timeout.");
                            debug!("Dropping socket");
                            drop(requester);
                            debug!("Creating new socket");
                            requester = context.socket(zmq::REQ).unwrap();
                            requester.set_linger(LINGER_PERIOD_MS).unwrap();
                            debug!("Linger: {:?}", requester.get_linger());
                            assert!(requester.connect(&addr).is_ok());
                        }
                    }
                    Err(msg) => error!("Polling Error: {}", msg),
                }
            }
        }
        info!("Stopping client thread");
    }
}
