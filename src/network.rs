use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::net::{Shutdown, TcpListener, TcpStream};

use crate::routing::id::Identifier;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Lookup(Identifier),
    Result(SocketAddr),
    Notify(SocketAddr),
    Ping,
    Pong,
}

#[derive(Debug)]
pub enum MessageError {
    IOError(std::io::Error),
    SerdeError(serde_cbor::Error),
}

impl From<std::io::Error> for MessageError {
    fn from(err: std::io::Error) -> Self {
        MessageError::IOError(err)
    }
}
impl From<serde_cbor::Error> for MessageError {
    fn from(err: serde_cbor::Error) -> Self {
        MessageError::SerdeError(err)
    }
}

pub fn send_message(msg: Message, addr: SocketAddr) -> Result<Message, MessageError> {
    let stream = &TcpStream::connect(addr)?;
    // send messages
    serde_cbor::to_writer(stream, &msg)?;
    stream.shutdown(Shutdown::Write)?;
    // read response
    let answer = serde_cbor::from_reader(stream)?;
    Ok(answer)
}

pub fn listen_for_messages(
    addr: SocketAddr,
    handler: impl Fn(Message) -> Option<Message>,
) -> Result<(), MessageError> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        match &stream {
            Ok(tcp_stream) => {
                let current_ref = || -> Result<(), MessageError> {
                    let msg = serde_cbor::from_reader(tcp_stream)?;
                    if let Some(resp) = handler(msg) {
                        serde_cbor::to_writer(tcp_stream, &resp)?;
                    }
                    Ok(())
                }();
                if let Err(err) = current_ref {
                    println!("error: {:?}", err)
                }
            }
            Err(err) => {
                println!("error: {:?}", err)
            }
        }
    }
    Ok(())
}
