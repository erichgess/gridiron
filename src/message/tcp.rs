use super::comm::Communicator;
use super::util;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

type Sender = mpsc::Sender<(usize, Vec<u8>)>;
type Receiver = mpsc::Receiver<(usize, Vec<u8>)>;

pub struct TcpHost {
    send_sink: mpsc::Sender<(usize, Vec<u8>)>,
    recv_src: mpsc::Receiver<Vec<u8>>,
    listen_thread: thread::JoinHandle<()>,
    send_thread: thread::JoinHandle<()>,
}

impl TcpHost {
    pub fn new(rank: usize, peers: Vec<SocketAddr>) -> Self {
        let (send_sink, send_src): (Sender, Receiver) = mpsc::channel();
        let peers_cpy = peers.clone();
        let send_thread = thread::spawn(move || {
            for (rank, message) in send_src {
                let mut stream = TcpStream::connect(peers_cpy[rank]).unwrap();
                stream.write_all(&message.len().to_le_bytes()).unwrap();
                stream.write_all(&message).unwrap();
            }
        });

        let (recv_sink, recv_src) = mpsc::channel();
        let listen_thread = thread::spawn(move || {
            let listener = TcpListener::bind(peers[rank]).unwrap();
            loop {
                let (mut stream, _) = listener.accept().unwrap(); // There is a race condition here
                let size = util::read_usize(&mut stream);
                let bytes = util::read_bytes_vec(&mut stream, size);
                match recv_sink.send(bytes) {
                    Ok(_) => (),
                    Err(_) => break,
                }
            }
        });

        TcpHost {
            send_sink,
            recv_src,
            send_thread,
            listen_thread,
        }
    }
}

pub struct TcpCommunicator {
    rank: usize,
    num_peers: usize,
    send_sink: Option<mpsc::Sender<(usize, Vec<u8>)>>,
    send_thread: Option<thread::JoinHandle<()>>,
}

impl TcpCommunicator {
    pub fn new(rank: usize, peers: Vec<SocketAddr>) -> Self {
        let num_peers = peers.len();
        let (send_sink, recv_sink): (Sender, Receiver) = mpsc::channel();
        let send_thread = thread::spawn(move || {
            for (rank, message) in recv_sink {
                let mut stream = TcpStream::connect(peers[rank]).unwrap();
                stream.write_all(&message.len().to_le_bytes()).unwrap();
                stream.write_all(&message).unwrap();
            }
        });
        Self {
            rank,
            num_peers,
            send_sink: Some(send_sink),
            send_thread: Some(send_thread),
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
        vec![]
    }
}

impl Drop for TcpCommunicator {
    fn drop(&mut self) {
        self.send_sink.take().unwrap();
        self.send_thread.take().unwrap().join().unwrap();
    }
}
