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
    pub fn new(rank: usize, peers: Vec<SocketAddr>) -> (Self, Sender, Receiver) {
        let (send_sink, send_src): (Sender, _) = crossbeam_channel::unbounded();
        let peers_cpy = peers.clone();
        let send_thread = thread::spawn(move || {
            for (rank, message) in send_src {
                info!("Sending Message");
                let mut attempts = 0;
                let mut sleep_ms = 1000;
                while attempts < 3 {
                    attempts += 1;
                    match TcpStream::connect(peers_cpy[rank]) {
                        Ok(mut stream) => {
                            stream.write_all(&message.len().to_le_bytes()).unwrap();
                            stream.write_all(&message).unwrap();
                        }
                        Err(msg) => {
                            error!("Send failed, retrying: {}", msg);
                            thread::sleep(std::time::Duration::from_millis(sleep_ms));
                            sleep_ms *= 2;
                        }
                    }
                }
            }
        });

        let (recv_sink, recv_src) = crossbeam_channel::unbounded();
        let listen_thread = thread::spawn(move || {
            let listener = TcpListener::bind(peers[rank]).unwrap();
            loop {
                let (mut stream, _) = listener.accept().unwrap(); // TODO: There is a race condition here
                let size = util::read_usize(&mut stream);
                let bytes = util::read_bytes_vec(&mut stream, size);
                match recv_sink.send(bytes) {
                    Ok(_) => (),
                    Err(_) => break,
                }
            }
        });

        (
            TcpHost {
                send_thread: Some(send_thread),
                listen_thread: Some(listen_thread),
            },
            send_sink,
            recv_src,
        )
    }

    pub fn join(&mut self) {
        self.send_thread.take().unwrap().join().unwrap()
    }
}

pub struct TcpCommunicator {
    rank: usize,
    num_peers: usize,
    send_sink: Option<crossbeam_channel::Sender<(usize, Vec<u8>)>>,
    recv_src: Option<crossbeam_channel::Receiver<Vec<u8>>>,
}

impl TcpCommunicator {
    pub fn new(
        rank: usize,
        peers: Vec<SocketAddr>,
        send_sink: crossbeam_channel::Sender<(usize, Vec<u8>)>,
        recv_src: crossbeam_channel::Receiver<Vec<u8>>,
    ) -> Self {
        let num_peers = peers.len();
        Self {
            rank,
            num_peers,
            send_sink: Some(send_sink),
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
        info!("Sending message");
        self.send_sink
            .as_ref()
            .unwrap()
            .send((rank, message))
            .unwrap()
    }

    fn recv(&self) -> Vec<u8> {
        self.recv_src.as_ref().unwrap().recv().unwrap()
    }
}

impl Drop for TcpCommunicator {
    fn drop(&mut self) {
        self.send_sink.take().unwrap();
        self.recv_src.take().unwrap();
    }
}
