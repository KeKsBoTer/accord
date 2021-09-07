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
            Message::Result(addr) => Ok(Neighbor::new(addr)),
            msg => Err(MessageError::UnexpectedResponse(msg, response)),
        }
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
    successor: Mutex<Option<Neighbor>>,

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
            successor: Mutex::new(Some(Neighbor::new(addr))),

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
            if let Some(n) = succ {
                let addr = n.find_successor(id).await?;
                todo!("get value");
                return Ok(None);
            }

            return Ok(None);
        }
    }

    pub async fn handle_message(&self, msg: Message) -> Result<Option<Message>, MessageError> {
        match msg {
            Message::Lookup(id) => {
                let responsible_node = self.find_successor(id).await?;
                Ok(Some(Message::Result(responsible_node.addr)))
            }
            Message::Notify(addr) => {
                self.notify(addr.into());
                Ok(None)
            }
            Message::Ping => Ok(Some(Message::Pong)),
            _ => panic!("this should not happen (incomming message: {:?})", msg),
        }
    }

    pub async fn join(&self, entry: SocketAddr) -> Result<(), MessageError> {
        let neighbor = Neighbor::new(entry);
        let mut pred = self.predecessor.lock().unwrap();
        *pred = None;
        let mut succ = self.successor.lock().unwrap();
        *succ = Some(neighbor.find_successor(self.id).await?);
        Ok(())
    }

    async fn find_successor(&self, id: Identifier) -> Result<Neighbor, MessageError> {
        if self.contains_id(id) {
            Ok(self.address.into())
        } else {
            self.successor
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .find_successor(id)
                .await
        }
    }

    async fn find_predecessor(&self, id: Identifier) -> Neighbor {
        todo!("find predecessor for an id")
        // find_successor(id).predecessor
    }

    async fn check_predecessor(&self) {
        if let Some(predecessor) = self.predecessor.lock().unwrap().as_mut() {
            let resp = network::send_message(Message::Ping, predecessor.addr).await;
            if resp.is_err() {
                // node is dead
                *predecessor = self.find_predecessor(predecessor.id).await;
            }
        }
    }

    fn notify(&self, other: Neighbor) {
        let mut pred = self.predecessor.lock().unwrap();
        if let Some(predecessor) = pred.as_mut() {
            if other.id > predecessor.id {
                *predecessor = other
            }
        } else {
            *pred = Some(other)
        }
    }

    async fn stabilize(&self) {
        todo!("check if I am the successor of my predecessor")
        // TODO notify predecessor that I am his successor
        // successor.get_predecessor()
    }

    pub fn neighbors(&self) -> Vec<SocketAddr> {
        match self.successor.lock().unwrap().as_ref() {
            Some(successor) => vec![successor.addr],
            None => Vec::new(),
        }
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
