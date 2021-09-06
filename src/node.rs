use std::collections::HashMap;
use std::hash::Hash;
use std::net::SocketAddr;

use super::com::Message;

#[derive(Debug, PartialEq)]
struct Neighbor {
    id: u64,
    addr: SocketAddr,
}

#[derive(Debug)]
pub struct Node<K: Hash, V> {
    pub address: SocketAddr,
    predecessor: Option<Neighbor>,
    successor: Option<Neighbor>,

    id: u64,
    store: HashMap<K, V>,
}

impl<K, V> Node<K, V>
where
    K: Hash,
{
    pub fn new(address: SocketAddr) -> Self {
        Node {
            address: address,
            predecessor: None,
            successor: None, // TODO put self in here

            id: 0, // TODO: hash id
            store: HashMap::<K, V>::new(),
        }
    }

    fn join(&self, other: Neighbor) {
        todo!("join network")
    }

    fn find_successor(&self, id: u64) -> u64 {
        // TODO change to only accept Lookup message
        todo!()
        // if (i am responsible for the id)
        //    return self
        // else
        //    return successor.find_successor(id)
    }

    fn find_predecessor(&self, id: u64) -> u64 {
        todo!("find predecessor for an id")
        // find_successor(id).predecessor
    }

    fn check_predecessor(&self) {
        todo!("check if predecessor has failed")
        // ping predecessor
        // if alive
        //      do nothing
        // else
        //     predecessor = find_predecessor(predecessor.id)
    }

    fn notify(&self, other: Neighbor) {
        todo!("this node is called. add as predecessor if range fits")
        // if other.end > predecessor.end
        //     predecessor = other
    }

    fn stabilize(&self) {
        todo!("check if I am the successor of my predecessor")
        // TODO notify predecessor that I am his successor
        // successor.get_predecessor()
    }
}
