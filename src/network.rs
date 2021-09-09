use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::routing::id::Identifier;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Message {
    Lookup(Identifier),
    LookupResult(SocketAddr),

    GetPredecessor,
    PredecessorResponse(Option<SocketAddr>),

    Notify(SocketAddr),
    Ping,
    Pong,
}

#[derive(Debug)]
pub enum MessageError {
    IOError(std::io::Error),
    SerdeError(serde_cbor::Error),
    UnexpectedResponse(Message, Option<Message>),
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

pub async fn send_message(msg: Message, addr: SocketAddr) -> Result<Option<Message>, MessageError> {
    let mut stream = TcpStream::connect(addr).await?;
    // send message
    let buf = serde_cbor::to_vec(&msg)?;
    stream.write_all(&buf).await?;
    stream.shutdown().await?;

    // read response
    let mut resp_buf = Vec::with_capacity(32);
    stream.read_to_end(&mut resp_buf).await?;
    if resp_buf.is_empty() {
        Ok(None)
    } else {
        let answer: Message = serde_cbor::from_slice(resp_buf.as_slice())?;
        Ok(Some(answer))
    }
}

pub async fn listen_for_messages<F, Fut>(addr: SocketAddr, handler: F) -> Result<(), MessageError>
where
    F: Fn(Message) -> Fut,
    Fut: Future<Output = Option<Message>>,
{
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (mut tcp_stream, _) = listener.accept().await?;

        // TODO error handling
        let mut send_buf = Vec::with_capacity(32);
        tcp_stream.read_to_end(&mut send_buf).await.unwrap();
        let msg: Message = serde_cbor::from_slice(send_buf.as_slice()).unwrap();

        if let Some(resp) = handler(msg).await {
            let buf = serde_cbor::to_vec(&resp).unwrap();
            tcp_stream.write_all(&buf).await.unwrap();
            tcp_stream.shutdown().await.unwrap();
        }
    }
}
