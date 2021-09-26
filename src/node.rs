use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::hash::Hash;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{collections::HashMap, sync::Mutex};
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

    async fn get_predecessor(&self) -> Result<Option<Neighbor>, MessageError> {
        let msg = Message::GetPredecessor;
        handle_message!(self.addr, msg, {
            Message::PredecessorResponse(neighbor) => neighbor
        })
    }

    async fn notify(&self, neighbor: Neighbor) -> Result<(), MessageError> {
        handle_message!(self.addr, Message::Notify(neighbor))
    }
}

#[derive(Debug)]
pub struct Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString,
    Value: Clone + FromStr + ToString,
    <Value as FromStr>::Err: fmt::Debug,
{
    pub address: SocketAddr,
    pub web_address: SocketAddr,
    predecessor: Mutex<Option<Neighbor>>,
    successor: Mutex<Neighbor>,

    pub id: Identifier,
    store: Mutex<HashMap<Key, Value>>,
}

impl<Key, Value> Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString,
    Value: Clone + FromStr + ToString,
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
            predecessor: Mutex::new(Some(Neighbor::new(predecessor, predecessor_ws))),
            successor: Mutex::new(Neighbor::new(successor, successor_ws)),

            id: addr.hash_id(),
            store: Mutex::new(HashMap::<Key, Value>::new()),
        }
    }

    fn contains_id(&self, id: Identifier) -> bool {
        let pred = self.predecessor.lock().unwrap();

        pred.map(|n| id.is_between(n.id, self.id)).unwrap_or(true) // TODO is this right?
    }

    // finds the value for a given key within the chord ring
    pub async fn lookup(&self, key: Key) -> Result<Option<Value>, MessageError> {
        let id = key.hash_id();
        if self.contains_id(id) {
            let value = self.store.lock().unwrap().get(&key).map(|n| n.clone());
            return Ok(value);
        } else {
            let succ = self.successor.lock().unwrap().clone();
            let addr = succ.find_successor(id).await?;
            let client = Client::new();

            let url: Uri = format!("http://{:}/storage/{:}", addr.web_addr, key.to_string())
                .parse()
                .unwrap();

            // TODO error handling
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
            Message::Notify(addr) => {
                self.notify(addr.into());
                Ok(None)
            }
            Message::GetPredecessor => {
                let pred = self.predecessor.lock().unwrap();
                let response = Message::PredecessorResponse(*pred);
                Ok(Some(response))
            }
            Message::Ping => Ok(Some(Message::Pong)),
            _ => panic!("this should not happen (incomming message: {:?})", msg),
        }
    }

    pub async fn join(&self, entry_node: SocketAddr) -> Result<(), MessageError> {
        let neighbor = Neighbor::new(entry_node, entry_node);
        let mut pred = self.predecessor.lock().unwrap();
        *pred = None;
        let new_succ = neighbor.find_successor(self.id).await?;
        let mut succ = self.successor.lock().unwrap();
        *succ = new_succ;
        Ok(())
    }

    async fn find_successor(&self, id: Identifier) -> Result<Neighbor, MessageError> {
        let succ = self.successor.lock().unwrap().clone();

        if self.contains_id(id) || succ.id == self.id {
            Ok(Neighbor::new(self.address, self.web_address))
        } else {
            succ.find_successor(id).await
        }
    }

    fn notify(&self, other: Neighbor) {
        let mut pred = self.predecessor.lock().unwrap();
        match pred.as_mut() {
            Some(predecessor) => {
                if other.id.is_between(predecessor.id, self.id) {
                    println!("[{:}] updated predecessor to {:}", self, other.addr);
                    *predecessor = other
                }
            }
            None => {
                println!("[{:}] updated predecessor to {:}", self, other.addr);
                *pred = Some(other)
            }
        }
    }

    pub async fn stabilize(&self) -> Result<(), MessageError> {
        let mut successor = self.successor.lock().unwrap();

        let predecessor = if self.id == successor.id {
            *self.predecessor.lock().unwrap()
        } else {
            successor.get_predecessor().await?
        };
        if let Some(x) = predecessor {
            if x.id.is_between(self.id, successor.id) {
                println!("[{:}|{:}] updated successor to {:}", self, self.id, x.addr);
                *successor = x;
            }
        }
        // the node does not need to message itself
        if self.id != successor.id {
            successor
                .notify(Neighbor::new(self.address, self.web_address))
                .await?;
        }
        Ok(())
    }

    pub fn neighbors(&self) -> Vec<SocketAddr> {
        let succ = self.successor.lock().unwrap();
        // TODO return web api port, not chord port
        vec![succ.web_addr]
    }

    pub async fn put(&self, key: Key, value: Value) -> Result<(), MessageError> {
        let id = key.hash_id();
        if !self.contains_id(id) {
            let succ = self.successor.lock().unwrap().clone();
            let addr = succ.find_successor(id).await?;
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
            // TODO error handling
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

impl<Key, Value> Display for Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString,
    Value: Clone + FromStr + ToString,
    <Value as FromStr>::Err: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:})", self.address))
    }
}
