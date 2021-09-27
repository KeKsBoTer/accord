use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::hash::Hash;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use warp::http;
use warp::hyper::{Client, Uri};

use crate::handle_message;
use crate::{
    network::{self, Message, MessageError},
    routing::id::{HashIdentifier, Identifier},
};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct Neighbor {
    id: Identifier,
    addr: SocketAddr,
    web_addr: SocketAddr,
}

impl Neighbor {
    fn new(addr: SocketAddr, web_addr: SocketAddr) -> Self {
        Neighbor {
            id: addr.hash_id(),
            addr: addr,
            web_addr: web_addr,
        }
    }

    async fn find_successor(&self, id: Identifier) -> Result<Neighbor, MessageError> {
        let msg = Message::Lookup(id);
        handle_message!(self.addr, msg, {
            Message::LookupResult(neighbor) => neighbor
        })
    }
}

#[derive(Debug)]
pub struct Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString + Send + Sync + 'static,
    Value: Clone + FromStr + ToString + Send + Sync + 'static,
    <Value as FromStr>::Err: fmt::Debug,
{
    pub address: SocketAddr,
    pub web_address: SocketAddr,
    predecessor: Neighbor,
    successor: Neighbor,

    id: Identifier,
    store: Mutex<HashMap<Key, Value>>,
}

impl<Key, Value> Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString + Send + Sync + 'static,
    Value: Clone + FromStr + ToString + Send + Sync + 'static,
    <Value as FromStr>::Err: fmt::Debug,
{
    pub fn new(
        addr: SocketAddr,
        web_addr: SocketAddr,
        predecessor: SocketAddr,
        predecessor_ws: SocketAddr,
        successor: SocketAddr,
        successor_ws: SocketAddr,
    ) -> Self {
        Node {
            address: addr,
            web_address: web_addr,
            predecessor: Neighbor::new(predecessor, predecessor_ws),
            successor: Neighbor::new(successor, successor_ws),

            id: addr.hash_id(),
            store: Mutex::new(HashMap::<Key, Value>::new()),
        }
    }

    fn contains_id(&self, id: Identifier) -> bool {
        id.is_between(self.predecessor.id, self.id)
    }

    // finds the value for a given key within the chord ring
    pub async fn lookup(&self, key: Key) -> Result<Option<Value>, MessageError> {
        let id = key.hash_id();
        if self.contains_id(id) {
            let value = self.store.lock().unwrap().get(&key).map(|n| n.clone());
            return Ok(value);
        } else {
            let addr = self.successor.find_successor(id).await?;
            let client = Client::new();

            let url: Uri = format!("http://{:}/storage/{:}", addr.web_addr, key.to_string())
                .parse()
                .unwrap();

            let res = client.get(url).await.unwrap();
            match res.status() {
                http::StatusCode::NOT_FOUND => Ok(None),
                http::StatusCode::OK => {
                    let body = warp::hyper::body::to_bytes(res).await.unwrap();
                    let body_str = String::from_utf8(body.to_vec()).unwrap();
                    let v = Value::from_str(body_str.as_str()).unwrap();
                    Ok(Some(v))
                }
                status => Err(MessageError::HTTPStatusError(status)),
            }
        }
    }

    pub async fn handle_message(&self, msg: Message) -> Result<Option<Message>, MessageError> {
        match msg {
            Message::Lookup(id) => {
                let responsible_node = self.find_successor(id).await?;
                Ok(Some(Message::LookupResult(responsible_node)))
            }
            _ => panic!("this should not happen (incomming message: {:?})", msg),
        }
    }

    async fn find_successor(&self, id: Identifier) -> Result<Neighbor, MessageError> {
        if self.contains_id(id) {
            Ok(Neighbor::new(self.address, self.web_address))
        } else {
            self.successor.find_successor(id).await
        }
    }

    pub fn neighbors(&self) -> Vec<SocketAddr> {
        vec![self.successor.web_addr]
    }

    pub async fn put(&self, key: Key, value: Value) -> Result<(), MessageError> {
        let id = key.hash_id();
        if !self.contains_id(id) {
            let addr = self.successor.find_successor(id).await?;
            let client = Client::new();

            let url: Uri = format!("http://{:}/storage/{:}", addr.web_addr, key.to_string())
                .parse()
                .unwrap();

            let payload = warp::hyper::body::Body::from(value.to_string());
            let req = http::Request::builder()
                .uri(url)
                .method(http::Method::PUT)
                .body(payload)
                .unwrap();
            let res = client.request(req).await.unwrap();
            return match res.status() {
                http::StatusCode::OK => Ok(()),
                status => Err(MessageError::HTTPStatusError(status)),
            };
        }
        self.store.lock().unwrap().insert(key, value);
        Ok(())
    }
}

pub async fn message_listener<Key, Value>(node: Arc<Node<Key, Value>>)
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString + Send + Sync + 'static,
    Value: Clone + FromStr + ToString + Send + Sync + 'static,
    <Value as FromStr>::Err: fmt::Debug,
{
    let listener = TcpListener::bind(node.address).await.unwrap();
    loop {
        let (mut tcp_stream, _) = listener.accept().await.unwrap();
        let tcp_chord_node = node.clone();
        tokio::spawn(async move {
            let mut send_buf = Vec::with_capacity(32);
            tcp_stream.read_to_end(&mut send_buf).await.unwrap();
            let msg: Message = serde_cbor::from_slice(send_buf.as_slice()).unwrap();

            if let Some(resp) = tcp_chord_node.handle_message(msg).await.unwrap() {
                let buf = serde_cbor::to_vec(&resp).unwrap();
                tcp_stream.write_all(&buf).await.unwrap();
                tcp_stream.shutdown().await.unwrap();
            }
        });
    }
}

impl<Key, Value> Display for Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString + Send + Sync + 'static,
    Value: Clone + FromStr + ToString + Send + Sync + 'static,
    <Value as FromStr>::Err: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:})", self.address))
    }
}
