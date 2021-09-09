use std::fmt::Display;
use std::hash::Hash;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Mutex};

use crate::{
    network::{self, Message, MessageError},
    routing::id::{HashIdentifier, Identifier},
};

#[derive(Debug, PartialEq, Clone, Copy)]
struct Neighbor {
    id: Identifier,
    addr: SocketAddr,
}

impl Neighbor {
    fn new(addr: SocketAddr) -> Self {
        Neighbor {
            id: addr.hash_id(),
            addr: addr,
        }
    }

    async fn find_successor(&self, id: Identifier) -> Result<Neighbor, MessageError> {
        let msg = Message::Lookup(id);
        let response = network::send_message(msg, self.addr).await?;
        match response {
            Some(msg) => match msg {
                Message::LookupResult(addr) => Ok(Neighbor::new(addr)),
                m => Err(MessageError::UnexpectedResponse(m, response)),
            },
            None => Err(MessageError::UnexpectedResponse(msg, None)),
        }
    }

    async fn get_predecessor(&self) -> Result<Option<Neighbor>, MessageError> {
        let msg = Message::GetPredecessor;
        let response = network::send_message(msg, self.addr).await?;
        match response {
            Some(resp) => match resp {
                Message::PredecessorResponse(addr) => Ok(addr.map(|a| Neighbor::new(a))),
                r => Err(network::MessageError::UnexpectedResponse(msg, Some(r))),
            },
            None => Err(network::MessageError::UnexpectedResponse(msg, None)),
        }
    }

    async fn notify(&self, addr: SocketAddr) -> Result<(), MessageError> {
        let msg = Message::Notify(addr);
        network::send_message(msg, self.addr).await?;
        Ok(())
    }
}

impl From<SocketAddr> for Neighbor {
    fn from(addr: SocketAddr) -> Self {
        Neighbor::new(addr)
    }
}

#[derive(Debug)]
pub struct Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier>,
    Value: Clone,
{
    pub address: SocketAddr,
    predecessor: Mutex<Option<Neighbor>>,
    successor: Mutex<Neighbor>,

    id: Identifier,
    store: Mutex<HashMap<Key, Value>>,
}

impl<Key, Value> Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier>,
    Value: Clone,
{
    pub fn new(addr: SocketAddr) -> Self {
        Node {
            address: addr,
            predecessor: Mutex::new(None),
            successor: Mutex::new(Neighbor::new(addr)),

            id: addr.hash_id(),
            store: Mutex::new(HashMap::<Key, Value>::new()),
        }
    }

    fn contains_id(&self, id: Identifier) -> bool {
        self.predecessor
            .lock()
            .unwrap()
            .as_ref()
            .map(|n| id.is_between(self.id, n.id))
            .unwrap_or(true) // TODO is this right?
    }

    // finds the value for a given key within the chord ring
    pub async fn lookup(&self, key: Key) -> Result<Option<Value>, MessageError> {
        let id = key.hash_id();
        if self.contains_id(id) {
            let value = self.store.lock().unwrap().get(&key).map(|n| n.clone());
            return Ok(value);
        } else {
            let succ = self.successor.lock().unwrap().clone();
            let _addr = succ.find_successor(id).await?;
            todo!("get value from addr");
        }
    }

    pub async fn handle_message(&self, msg: Message) -> Result<Option<Message>, MessageError> {
        match msg {
            Message::Lookup(id) => {
                let responsible_node = self.find_successor(id).await?;
                Ok(Some(Message::LookupResult(responsible_node.addr)))
            }
            Message::Notify(addr) => {
                self.notify(addr.into());
                Ok(None)
            }
            Message::GetPredecessor => {
                let pred = self.predecessor.lock().unwrap();
                let response = Message::PredecessorResponse(pred.map(|n| n.addr));
                Ok(Some(response))
            }
            Message::Ping => Ok(Some(Message::Pong)),
            _ => panic!("this should not happen (incomming message: {:?})", msg),
        }
    }

    pub async fn join(&self, entry_node: SocketAddr) -> Result<(), MessageError> {
        let neighbor = Neighbor::new(entry_node);
        let mut pred = self.predecessor.lock().unwrap();
        *pred = None;
        let new_succ = neighbor.find_successor(self.id).await?;
        let mut succ = self.successor.lock().unwrap();
        *succ = new_succ;
        Ok(())
    }

    async fn find_successor(&self, id: Identifier) -> Result<Neighbor, MessageError> {
        if self.contains_id(id) {
            Ok(self.address.into())
        } else {
            let succ = self.successor.lock().unwrap().clone();
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

        // TODO check if this is right?
        // if the successor is the node itself and we just changed the predecessor
        // then the successor should be the predecessor (only two nodes in network)
        // otherwise the successor for the first node, that created the network,
        // is never updated.
        let mut succ = self.successor.lock().unwrap();
        if succ.id == self.id && pred.is_some() {
            println!("[{:}] updated successor to {:}", self, other.addr);
            *succ = pred.unwrap().clone()
        }
    }

    pub async fn stabilize(&self) -> Result<(), MessageError> {
        let mut successor = self.successor.lock().unwrap();
        if self.id == successor.id {
            // no need to send a message
            return Ok(());
        }
        if let Some(x) = successor.get_predecessor().await? {
            if x.id.is_between(self.id, successor.id) {
                println!("[{:}] updated successor to {:}", self, x.addr);
                *successor = x;
            }
        }
        successor.notify(self.address).await?;
        Ok(())
    }

    pub fn neighbors(&self) -> Vec<SocketAddr> {
        let succ = self.successor.lock().unwrap();
        // TODO return web api port, not chord port
        vec![succ.addr]
    }

    pub async fn put(&self, key: Key, value: Value) -> Result<(), MessageError> {
        let id = key.hash_id();
        if !self.contains_id(id) {
            todo!("tried to insert key with id '{:}' into wrong node", id)
        }
        self.store.lock().unwrap().insert(key, value);
        Ok(())
    }
}

impl<Key, Value> Display for Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier>,
    Value: Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("({:}|{:x})", self.address, u64::from(self.id)))
    }
}
