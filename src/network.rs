use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use warp::http;

use crate::node::Neighbor;
use crate::routing::id::Identifier;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Message {
    Lookup(Identifier),
    LookupResult(Neighbor),

    GetPredecessor,
    PredecessorResponse(Option<Neighbor>),

    GetSuccessor,
    SuccessorResponse(Neighbor),

    // message sent to successor when a node leaves
    LeavePredecessor(Option<Neighbor>),
    // message sent to predecessor when a node leaves
    LeaveSuccessor(Neighbor),

    Notify(Neighbor),

    Ping,
    Pong,
}

#[derive(Debug)]
pub enum MessageError {
    IOError(std::io::Error),
    SerdeError(serde_cbor::Error),
    UnexpectedResponse(Message, Option<Message>),
    AllSuccessorsDead(SocketAddr),
    HTTPStatusError(http::StatusCode),
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

#[macro_export]
macro_rules! handle_message {
    // return error if message is not handled by given pattern matching
    // e.g.
    // handle_message!(self.addr, msg, {
    //    Message::LookupResult(addr) => Neighbor::new(addr)
    // })
    // returns an error if response is not of type Message::LookupResult
    ($addr:expr , $msg: expr,{ $($p:pat => $handle:expr)+}) => {{
        let response = network::send_message($msg, $addr).await?;
        match response {
            Some(resp) => match resp {
                $(
                    $p => Ok($handle),
                )+
                r => Err(network::MessageError::UnexpectedResponse($msg, Some(r))),
            },
            None => Err(network::MessageError::UnexpectedResponse($msg, None)),
        }
    }};

    // no answer expected, return error if answer is not None
    ($addr:expr , $msg: expr) => {{
        let msg = $msg;
        let response = network::send_message(msg, $addr).await?;
        if response.is_some(){
            Err(network::MessageError::UnexpectedResponse(msg, response))
        }else{
            Ok(())
        }
    }};
}
