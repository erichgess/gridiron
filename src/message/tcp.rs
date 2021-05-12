use std::{
    collections::HashMap,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
};
use std::{io::prelude::*, thread::JoinHandle};
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

use log::{error, info};

use crate::message::{backoff::Retry, ordered::Envelope};

use super::{backoff::ExponentialBackoff, util};

const CXN_R_TIMEOUT_MS: Option<Duration> = None;
const CXN_W_TIMEOUT_MS: Option<Duration> = None;
const RETRY_WAIT_MS: Duration = Duration::from_millis(250);
const RETRY_MAX_WAIT_MS: Duration = Duration::from_millis(5000);

pub(super) type Iteration = usize;
type Sender = crossbeam_channel::Sender<(usize, Iteration, Vec<u8>)>;

pub struct TcpHost {
    shutting_down: Arc<AtomicBool>,
    _listen_thread: Option<thread::JoinHandle<()>>,
    send_thread: Option<thread::JoinHandle<()>>,
    receiver_wg: Arc<(Mutex<usize>, Condvar)>,
}

impl TcpHost {
    pub fn new(
        rank: usize,
        peers: Vec<SocketAddr>,
    ) -> (
        TcpHost,
        crossbeam_channel::Receiver<Envelope>,
        crossbeam_channel::Sender<(usize, usize, Vec<u8>)>,
    ) {
        let shutdown_signal = Arc::new(AtomicBool::new(false));

        let (send_sink, send_src): (Sender, _) = crossbeam_channel::unbounded();
        let send_thread =
            Self::start_serial_sender(peers.clone(), send_src, Arc::clone(&shutdown_signal));

        let (recv_sink, recv_src) = crossbeam_channel::unbounded();
        let wg = Arc::new((Mutex::new(0), Condvar::new()));
        let listen_thread = Self::start_listener(
            peers[rank],
            recv_sink.clone(),
            Arc::clone(&shutdown_signal),
            Arc::clone(&wg),
        );

        (
            TcpHost {
                shutting_down: shutdown_signal,
                send_thread: Some(send_thread),
                _listen_thread: Some(listen_thread),
                receiver_wg: wg,
            },
            recv_src,
            send_sink,
        )
    }

    pub fn shutdown(mut self) {
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::SeqCst);
        info!("Shutting down TCP host...");

        info!("Waiting for Sender to shutdown...");
        self.send_thread.take().unwrap().join().unwrap();
        info!("Sender shutdown");

        info!("Waiting for Receivers to shutdown...");
        let (lock, cvar) = &*self.receiver_wg;
        let mut receivers = lock.lock().unwrap();
        while *receivers > 0 {
            receivers = cvar.wait(receivers).unwrap();
        }
        info!("Receivers shutdown");

        info!("TCP host shutdown");
    }

    fn start_serial_sender(
        peers: Vec<SocketAddr>,
        send_src: crossbeam_channel::Receiver<(usize, Iteration, Vec<u8>)>,
        shutdown_signal: Arc<AtomicBool>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut table: HashMap<usize, TcpStream> = HashMap::new();

            for (rank, iteration, message) in &send_src {
                if !table.contains_key(&rank) {
                    table.insert(
                        rank,
                        Self::connect_with_retry(peers[rank], RETRY_WAIT_MS, RETRY_MAX_WAIT_MS)
                            .unwrap(),
                    );
                }
                let cxn = table.get_mut(&rank).unwrap();

                let message = rmp_serde::to_vec(&Envelope {
                    iteration,
                    data: message,
                })
                .unwrap();
                let msg_sz = message.len();

                // TODO: This is getting better.  Next step is to use the connection state to determine whether to delay and resend or to reconnect
                while let Some(Err(e)) =
                    ExponentialBackoff::new(RETRY_WAIT_MS, RETRY_MAX_WAIT_MS, 2)
                        .take(3)
                        .retry(
                            || {
                                cxn.write_all(&msg_sz.to_le_bytes())
                                    .and_then(|()| cxn.write_all(&message))
                                /*
                                The Ack mechanism was killing Linux performance.  Commented this out for now.
                                .and_then(|()| Self::read_ack(cxn))
                                .and_then(|ack|
                                    match ack {
                                        Ack::Accept(bytes_read) if bytes_read == msg_sz => Ok(()),
                                        Ack::Accept(bytes_read) =>
                                        panic!("Bytes read by receiver did not match bytes sent by this node.  Sent {} bytes but receiver Acked {} bytes", msg_sz, bytes_read),
                                    }
                                )*/
                            },
                            |e, d| {
                                error!("Send failed: {}", e);
                                info!("Retrying in {}ms", d.as_millis());
                                thread::sleep(d);
                            },
                        )
                {
                    error!("Failed to send message to {}: {}", peers[rank], e);
                    if shutdown_signal.load(Ordering::SeqCst) {
                        // Note: if there are a lot of outgoing messages and all peers are down, then this could take awhile
                        info!("Shutdown signal received, will drop this message");
                        info!(
                            "There are {} messages remaining in the channel",
                            &send_src.len()
                        );
                        break;
                    } else {
                        info!("Reconnecting to {}", peers[rank]);
                        *cxn =
                            Self::connect_with_retry(peers[rank], RETRY_WAIT_MS, RETRY_MAX_WAIT_MS)
                                .unwrap();
                    }
                }
            }

            info!("Stopped Sending Messages");
        })
    }

    fn start_listener(
        addr: SocketAddr,
        recv_sink: crossbeam_channel::Sender<Envelope>,
        shutdown_signal: Arc<AtomicBool>,
        receiver_wg: Arc<(Mutex<usize>, Condvar)>,
    ) -> thread::JoinHandle<()> {
        let listener = TcpListener::bind(addr).unwrap();
        thread::spawn(move || {
            info!("Listening to: {}", addr);
            loop {
                let (stream, remote) = listener.accept().unwrap();

                if shutdown_signal.load(Ordering::SeqCst) {
                    info!("Received connection attempt but this service is shutting down.  Rejecting and stopping the Listener...");
                    break;
                }

                {
                    let (receivers_lock, _) = &*receiver_wg;
                    *receivers_lock.lock().unwrap() += 1;
                }
                Self::handle_connection(
                    stream,
                    remote,
                    recv_sink.clone(),
                    Arc::clone(&shutdown_signal),
                    Arc::clone(&receiver_wg),
                );
            }
        })
    }

    fn handle_connection(
        mut stream: TcpStream,
        remote: SocketAddr,
        recv_sink: crossbeam_channel::Sender<Envelope>,
        shutdown_signal: Arc<AtomicBool>,
        receiver_wg: Arc<(Mutex<usize>, Condvar)>,
    ) -> JoinHandle<Result<(), io::Error>> {
        info!("Receiving connection from {}", remote);
        stream.set_read_timeout(CXN_R_TIMEOUT_MS).unwrap();
        stream.set_write_timeout(CXN_W_TIMEOUT_MS).unwrap();
        info!(
            "Read timeout {:?}ms.  Write timeout {:?}ms",
            CXN_R_TIMEOUT_MS.map(|t| t.as_millis()),
            CXN_W_TIMEOUT_MS.map(|t| t.as_millis()),
        );

        thread::spawn(move || {
            let status = loop {
                let status = util::read_usize(&mut stream)
                    .and_then(|size| util::read_bytes_vec(&mut stream, size))
                    .and_then(|bytes| {
                        let num_bytes = bytes.len();
                        rmp_serde::from_slice::<Envelope>(&bytes)
                            .map_err(|msg| io::Error::new(io::ErrorKind::InvalidData, msg))
                            .and_then(|envelope| {
                                recv_sink
                                    .send(envelope)
                                    .map(|()| num_bytes)
                                    .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))
                            })
                    })
                    //.and_then(|size| Self::write_ack(&mut stream, Ack::Accept(size)))
                    // TODO: if a reading error happens then send back a Failure message to the sender
                    .map_err(|e| {
                        io::Error::new(
                            e.kind(),
                            format!("Connection from {} failed: {}", remote, e),
                        )
                    });

                match status {
                    Ok(_) => (),
                    Err(e) => break Err(e),
                }

                if shutdown_signal.load(Ordering::SeqCst) {
                    break Ok(());
                }
            };

            match &status {
                Ok(()) => (),
                Err(e) => error!("{}", e),
            }

            info!("Stopping receiver for {}...", remote);
            let (lock, cvar) = &*receiver_wg;
            let mut receivers = lock.lock().unwrap();
            *receivers -= 1;
            cvar.notify_all();
            info!("Stopped receiver for {}", remote);

            status
        })
    }

    fn write_ack(stream: &mut TcpStream, ack: Ack) -> Result<(), io::Error> {
        rmp_serde::encode::write(stream, &ack).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn read_ack(stream: &mut TcpStream) -> Result<Ack, io::Error> {
        rmp_serde::decode::from_read(stream).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn connect_with_retry(
        addr: SocketAddr,
        initial_wait: Duration,
        max_wait: Duration,
    ) -> Option<TcpStream> {
        println!("Connecting to {}", addr);
        let mut with_retries = ExponentialBackoff::new(initial_wait, max_wait, 2);

        with_retries
            .retry(
                || TcpStream::connect(&addr),
                |e, d| {
                    error!("Failed to connect to {}", e);
                    info!("Retrying in {}ms", d.as_millis());
                    thread::sleep(d);
                },
            )
            .map(|r| r.unwrap())
            .map(|s| {
                s.set_read_timeout(CXN_R_TIMEOUT_MS).unwrap();
                s.set_write_timeout(CXN_W_TIMEOUT_MS).unwrap();
                info!(
                    "Connected to {} with: Read timeout {:?}ms.  Write timeout {:?}ms",
                    addr,
                    CXN_R_TIMEOUT_MS.map(|t| t.as_millis()),
                    CXN_W_TIMEOUT_MS.map(|t| t.as_millis()),
                );
                s
            })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
enum Ack {
    Accept(usize),
}
