use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::{net::SocketAddr, ops::Range};

use super::bucket::Bucket;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message<const N: u64> {
    Lookup(Bucket<N>, SocketAddr),
    Result(bool),
    Notify(Bucket<N>),
    Ping,
    Pong,
}

pub struct MessageSocket<const N: u64>(UdpSocket);

impl<const N: u64> MessageSocket<N> {
    pub fn bind(addr: SocketAddr) -> std::io::Result<MessageSocket<N>> {
        UdpSocket::bind(addr).and_then(|socket| Ok(MessageSocket(socket)))
    }

    pub fn send_to(&self, msg: Message<N>, addr: SocketAddr) -> std::io::Result<usize> {
        let buf = serde_cbor::to_vec::<Message<N>>(&msg).unwrap();
        self.0.send_to(buf.as_slice(), addr)
    }

    pub fn recv_from(&self) -> std::io::Result<(Message<N>, SocketAddr)> {
        let mut buf = [0; 32];
        let (n, addr) = self.0.recv_from(&mut buf)?;
        let msg = serde_cbor::from_slice(&buf[..n]).unwrap();
        Ok((msg, addr))
    }
}
