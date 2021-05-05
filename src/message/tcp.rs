use log::{error, info};

use super::comm::Communicator;
use super::util;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;

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
        let send_thread = Self::start_sender(peers.clone(), send_src);

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

    fn start_sender(
        peers: Vec<SocketAddr>,
        send_src: crossbeam_channel::Receiver<(usize, Vec<u8>)>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            for (rank, message) in send_src {
                let mut sleep_ms = 250;
                loop {
                    match TcpStream::connect(peers[rank]) {
                        Ok(mut stream) => {
                            stream.write_all(&message.len().to_le_bytes()).unwrap();
                            stream.write_all(&message).unwrap();
                            break;
                        }
                        Err(msg) => {
                            error!("Send failed: {}", msg);
                            info!("Retrying in {}ms", sleep_ms);
                            thread::sleep(std::time::Duration::from_millis(sleep_ms));
                            sleep_ms = if sleep_ms < 5000 { 2 * sleep_ms } else { 5000 };
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
                let (mut stream, _) = listener.accept().unwrap(); // TODO: There is a race condition here
                Self::handle_connection(&mut stream, recv_sink.clone());
            }
        })
    }

    fn handle_connection(stream: &mut TcpStream, recv_sink: crossbeam_channel::Sender<Vec<u8>>) {
        loop {
            let size = util::read_usize(stream);
            let bytes = util::read_bytes_vec(stream, size);
            match recv_sink.send(bytes) {
                Ok(_) => (),
                Err(msg) => {
                    error!("Connection failed: {}", msg);
                    break;
                }
            }
        }
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
