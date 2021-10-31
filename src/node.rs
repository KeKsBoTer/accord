use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::hash::Hash;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::sync::Mutex;
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
    pub addr: SocketAddr,
    pub web_addr: SocketAddr,
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

    // tell a node that its predecessor left the network
    // and the given node is his new predecessor
    async fn leave_predecessor(&self, new_predecessor: Neighbor) -> Result<(), MessageError> {
        handle_message!(self.addr, Message::LeavePredecessor(new_predecessor))
    }

    // tell a node that its successor left the network
    // and the given node is his new successor
    async fn leave_successor(&self, new_successor: Neighbor) -> Result<(), MessageError> {
        handle_message!(self.addr, Message::LeaveSuccessor(new_successor))
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
    pub predecessor: Mutex<Option<Neighbor>>,
    pub successor: Mutex<Neighbor>,

    pub id: Identifier,
    store: Mutex<HashMap<Key, Value>>,
}

impl<Key, Value> Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier> + ToString,
    Value: Clone + FromStr + ToString,
    <Value as FromStr>::Err: fmt::Debug,
{
    pub fn new(addr: SocketAddr, web_addr: SocketAddr) -> Self {
        Node {
            address: addr,
            web_address: web_addr,
            predecessor: Mutex::new(None),
            successor: Mutex::new(Neighbor::new(addr, web_addr)),

            id: addr.hash_id(),
            store: Mutex::new(HashMap::<Key, Value>::new()),
        }
    }

    async fn contains_id(&self, id: Identifier) -> bool {
        self.predecessor
            .lock()
            .await
            .map(|n| id.is_between(self.id, n.id))
            .unwrap_or(true)
    }

    // finds the value for a given key within the chord ring
    pub async fn lookup(&self, key: Key) -> Result<Option<Value>, MessageError> {
        let id = key.hash_id();
        if self.contains_id(id).await {
            let value = self.store.lock().await.get(&key).map(|n| n.clone());
            return Ok(value);
        } else {
            let succ = self.successor.lock().await.clone();
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
                // TODO failes
                let responsible_node = self.find_successor(id).await?;
                Ok(Some(Message::LookupResult(responsible_node)))
            }
            Message::Notify(addr) => {
                self.notify(addr.into()).await;
                Ok(None)
            }
            Message::GetPredecessor => {
                let pred = self.predecessor.lock().await;
                let response = Message::PredecessorResponse(*pred);
                Ok(Some(response))
            }
            Message::LeaveSuccessor(new_succecessor) => {
                // our successor left so we need to update it to the
                // new given one
                let mut successor = self.successor.lock().await;
                *successor = new_succecessor;
                Ok(None)
            }
            Message::LeavePredecessor(new_predecessor) => {
                // our predecessor left so we need to update it to the
                // new given one
                let mut pred = self.predecessor.lock().await;
                *pred = Some(new_predecessor);
                Ok(None)
            }
            Message::Ping => Ok(Some(Message::Pong)),
            _ => panic!("this should not happen (incomming message: {:?})", msg),
        }
    }

    pub async fn join(&self, entry_node: SocketAddr) -> Result<(), MessageError> {
        if entry_node == self.address {
            // node does not need to join itself
            return Ok(());
        }
        let neighbor = Neighbor::new(entry_node, entry_node);
        let mut pred = self.predecessor.lock().await;
        *pred = None;
        // TODO can fail if entry_node == self
        let new_succ = neighbor.find_successor(self.id).await?;
        let mut succ = self.successor.lock().await;
        *succ = new_succ;
        Ok(())
    }

    pub async fn leave(&self) -> Result<(), MessageError> {
        let pred = self.predecessor.lock().await;
        let succ = self.successor.lock().await;
        if let Some(p) = pred.as_ref() {
            p.leave_successor(succ.clone()).await?;
            succ.leave_predecessor(p.clone()).await?;
        }

        let mut pred = self.predecessor.lock().await;
        *pred = None;
        let mut succ = self.predecessor.lock().await;
        *succ = Some(Neighbor::new(self.address, self.web_address));
        Ok(())
    }

    async fn find_successor(&self, id: Identifier) -> Result<Neighbor, MessageError> {
        if self.contains_id(id).await {
            Ok(Neighbor::new(self.address, self.web_address))
        } else {
            let succ = self.successor.lock().await.clone();
            if succ.id == self.id {
                Ok(Neighbor::new(self.address, self.web_address))
            } else {
                succ.find_successor(id).await
            }
        }
    }

    async fn notify(&self, other: Neighbor) {
        let mut pred = self.predecessor.lock().await;
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
        let mut successor = self.successor.lock().await;

        let predecessor = if self.id == successor.id {
            *self.predecessor.lock().await
        } else {
            successor.get_predecessor().await?
        };
        if let Some(x) = predecessor {
            if x.id.is_between(self.id, successor.id) {
                println!("[{:}] updated successor to {:}", self, x.addr);
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

    pub async fn neighbors(&self) -> Vec<SocketAddr> {
        let succ = self.successor.lock().await;
        vec![succ.web_addr]
    }

    pub async fn put(&self, key: Key, value: Value) -> Result<(), MessageError> {
        let id = key.hash_id();
        if !self.contains_id(id).await {
            let succ = self.successor.lock().await.clone();
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
        self.store.lock().await.insert(key, value);
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
        f.write_str(&self.address.to_string())
    }
}
