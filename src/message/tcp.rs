use std::{collections::HashMap, io, thread};
use std::{io::prelude::*, thread::JoinHandle};
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

use log::{error, info};

use super::{backoff::ExponentialBackoff, comm::Communicator, util};

const CXN_R_TIMEOUT_MS: Duration = Duration::from_millis(250);
const CXN_W_TIMEOUT_MS: Duration = Duration::from_millis(250);
const RETRY_WAIT_MS: Duration = Duration::from_millis(250);
const RETRY_MAX_WAIT_MS: Duration = Duration::from_millis(5000);

type Sender = crossbeam_channel::Sender<(usize, Vec<u8>)>;
type Receiver = crossbeam_channel::Receiver<Vec<u8>>;

pub struct TcpHost {
    listen_thread: Option<thread::JoinHandle<()>>,
    send_thread: Option<thread::JoinHandle<()>>,
}

impl TcpHost {
    pub fn new(
        rank: usize,
        peers: Vec<SocketAddr>,
    ) -> (Self, Sender, crossbeam_channel::Sender<Vec<u8>>, Receiver) {
        let (send_sink, send_src): (Sender, _) = crossbeam_channel::unbounded();
        let send_thread = Self::start_serial_sender(peers.clone(), send_src);

        let (recv_sink, recv_src) = crossbeam_channel::unbounded();
        let listen_thread = Self::start_listener(peers[rank], recv_sink.clone());

        (
            TcpHost {
                send_thread: Some(send_thread),
                listen_thread: Some(listen_thread),
            },
            send_sink,
            recv_sink,
            recv_src,
        )
    }

    pub fn join(&mut self) {
        self.send_thread.take().unwrap().join().unwrap()
    }

    fn start_serial_sender(
        peers: Vec<SocketAddr>,
        send_src: crossbeam_channel::Receiver<(usize, Vec<u8>)>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut table: HashMap<usize, TcpStream> = HashMap::new();

            for (rank, message) in send_src {
                if !table.contains_key(&rank) {
                    table.insert(
                        rank,
                        Self::connect_with_retry(peers[rank], RETRY_WAIT_MS, RETRY_MAX_WAIT_MS)
                            .unwrap(),
                    );
                }
                let client = table.get_mut(&rank).unwrap();

                loop {
                    // TODO: This will create a tight loop, don't use connect to create the backoff
                    // Need to distinguish retrying from a failed said and retrying from a broken connection
                    let msg_sz = message.len();
                    match client
                        .write_all(&msg_sz.to_le_bytes())
                        .and_then(|()| client.write_all(&message))
                        .and_then(|()| {
                            util::read_usize(client).and_then(|ack| {
                                if ack != msg_sz {
                                    panic!("Bytes read by receiver did not match bytes sent by this node.  Sent {} bytes but receiver Acked {} bytes", msg_sz, ack)
                                }
                                Ok(())
                            })
                        }) {
                        Ok(()) => break,
                        Err(msg) => {
                            error!("Failed to send message to {}: {}", peers[rank], msg);
                            *client = Self::connect_with_retry(peers[rank], RETRY_WAIT_MS, RETRY_MAX_WAIT_MS).unwrap();
                        }
                    }
                }
            }
        })
    }

    fn start_listener(
        addr: SocketAddr,
        recv_sink: crossbeam_channel::Sender<Vec<u8>>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            info!("Listening to: {}", addr);
            let listener = TcpListener::bind(addr).unwrap();
            loop {
                let (stream, remote) = listener.accept().unwrap(); // TODO: There is a race condition here
                Self::handle_connection(stream, remote, recv_sink.clone());
            }
        })
    }

    fn handle_connection(
        mut stream: TcpStream,
        remote: SocketAddr,
        recv_sink: crossbeam_channel::Sender<Vec<u8>>,
    ) -> JoinHandle<Result<(), std::io::Error>> {
        info!("Receiving connection from {}", remote);
        stream.set_read_timeout(Some(CXN_R_TIMEOUT_MS)).unwrap();
        stream.set_write_timeout(Some(CXN_W_TIMEOUT_MS)).unwrap();
        thread::spawn(move || loop {
            util::read_usize(&mut stream)
                .and_then(|size| util::read_bytes_vec(&mut stream, size))
                .and_then(|bytes| {
                    let num_bytes = bytes.len();
                    recv_sink
                        .send(bytes)
                        .map(|()| num_bytes)
                        .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))
                })
                .and_then(|size| stream.write(&size.to_le_bytes()).map(|_| ()))
                .map_err(|e| {
                    std::io::Error::new(
                        e.kind(),
                        format!("Connection from {} failed: {}", remote, e),
                    )
                })?
        })
    }

    fn connect_with_retry(
        addr: SocketAddr,
        initial_wait: Duration,
        max_wait: Duration,
    ) -> Option<TcpStream> {
        println!("Connecting...");
        let mut with_retries = ExponentialBackoff::new(initial_wait, max_wait, 2, None);

        with_retries.find_map(|sleep| match TcpStream::connect(&addr) {
            Ok(s) => {
                s.set_read_timeout(Some(CXN_R_TIMEOUT_MS)).unwrap();
                s.set_write_timeout(Some(CXN_W_TIMEOUT_MS)).unwrap();
                Some(s)
            }
            Err(msg) => {
                println!("Connect Failed: {}", msg);
                thread::sleep(sleep);
                None
            }
        })
    }
}

/////////////////////////////////////////////////////
/////////////////////////////////////////////////////
/////////////////////////////////////////////////////
/////////////////////////////////////////////////////

pub struct TcpCommunicator {
    rank: usize,
    num_peers: usize,
    send_sink: Option<crossbeam_channel::Sender<(usize, Vec<u8>)>>,
    recv_sink: Option<crossbeam_channel::Sender<Vec<u8>>>,
    recv_src: Option<crossbeam_channel::Receiver<Vec<u8>>>,
}

impl TcpCommunicator {
    pub fn new(
        rank: usize,
        peers: Vec<SocketAddr>,
        send_sink: crossbeam_channel::Sender<(usize, Vec<u8>)>,
        recv_sink: crossbeam_channel::Sender<Vec<u8>>,
        recv_src: crossbeam_channel::Receiver<Vec<u8>>,
    ) -> Self {
        let num_peers = peers.len();
        Self {
            rank,
            num_peers,
            send_sink: Some(send_sink),
            recv_sink: Some(recv_sink),
            recv_src: Some(recv_src),
        }
    }
}

impl Communicator for TcpCommunicator {
    fn rank(&self) -> usize {
        self.rank
    }

    fn size(&self) -> usize {
        self.num_peers
    }

    fn send(&self, rank: usize, message: Vec<u8>) {
        self.send_sink
            .as_ref()
            .unwrap()
            .send((rank, message))
            .unwrap()
    }

    fn recv(&self) -> Vec<u8> {
        self.recv_src.as_ref().unwrap().recv().unwrap()
    }

    fn requeue_recv(&self, bytes: Vec<u8>) {
        self.recv_sink.as_ref().unwrap().send(bytes).unwrap();
    }
}

impl Drop for TcpCommunicator {
    fn drop(&mut self) {
        self.send_sink.take().unwrap();
        self.recv_src.take().unwrap();
    }
}
