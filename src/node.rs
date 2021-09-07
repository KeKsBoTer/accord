use std::collections::HashMap;
use std::hash::Hash;
use std::net::SocketAddr;

use crate::{
    network::{self, Message},
    routing::id::{HashIdentifier, Identifier},
};

#[derive(Debug, PartialEq)]
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

    async fn find_successor(&self, id: Identifier) -> Neighbor {
        let response = network::send_message(Message::Lookup(id), self.addr)
            .await
            .unwrap();
        match response {
            Message::Result(addr) => addr.into(),
            msg => panic!("unexpected response: {:?}", msg),
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
{
    pub address: SocketAddr,
    predecessor: Option<Neighbor>,
    successor: Option<Neighbor>,

    id: Identifier,
    store: HashMap<Key, Value>,
}

impl<Key, Value> Node<Key, Value>
where
    Key: Eq + Hash + HashIdentifier<Identifier>,
{
    pub fn new(addr: SocketAddr) -> Self {
        Node {
            address: addr,
            predecessor: None,
            successor: Some(Neighbor::new(addr)),

            id: addr.hash_id(),
            store: HashMap::<Key, Value>::new(),
        }
    }

    fn contains_id(&self, id: Identifier) -> bool {
        match &self.predecessor {
            Some(p) => id.is_between(self.id, p.id),
            None => true, // TODO is this right?
        }
    }

    // finds the value for a given key within the chord ring
    pub async fn lookup(&self, key: Key) -> Option<&Value> {
        let id = key.hash_id();
        if self.contains_id(id) {
            return self.store.get(&key);
        } else {
            // find responsible node
            let successor_addr = self
                .successor
                .as_ref()
                .and_then(|s| Some(s.find_successor(id)));
            if let Some(addr) = successor_addr {
                todo!("get value");
                return None;
            }
            return None;
        }
    }

    pub async fn handle_message(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::Lookup(id) => Some(Message::Result(self.find_successor(id).await.addr)),
            Message::Notify(addr) => {
                self.notify(addr.into());
                None
            }
            Message::Ping => Some(Message::Pong),
            _ => panic!("this should not happen (incomming message: {:?})", msg),
        }
    }

    pub async fn join(&mut self, entry: SocketAddr) {
        let neighbor = Neighbor::new(entry);
        self.predecessor = None;
        self.successor = Some(neighbor.find_successor(self.id).await);
    }

    async fn find_successor(&self, id: Identifier) -> Neighbor {
        if self.contains_id(id) {
            self.address.into()
        } else {
            self.successor.as_ref().unwrap().find_successor(id).await
        }
    }

    async fn find_predecessor(&self, id: Identifier) -> Neighbor {
        todo!("find predecessor for an id")
        // find_successor(id).predecessor
    }

    async fn check_predecessor(&mut self) {
        if let Some(predecessor) = &self.predecessor {
            let resp = network::send_message(Message::Ping, predecessor.addr).await;
            if resp.is_err() {
                // node is dead
                self.predecessor = Some(self.find_predecessor(predecessor.id).await);
            }
        }
    }

    fn notify(&mut self, other: Neighbor) {
        if let Some(predecessor) = &self.predecessor {
            if other.id > predecessor.id {
                self.predecessor = Some(other)
            }
        } else {
            self.predecessor = Some(other)
        }
    }

    async fn stabilize(&self) {
        todo!("check if I am the successor of my predecessor")
        // TODO notify predecessor that I am his successor
        // successor.get_predecessor()
    }
}
