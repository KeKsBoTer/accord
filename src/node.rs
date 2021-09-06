use std::collections::HashMap;
use std::hash::Hash;
use std::net::SocketAddr;
use std::ops::Range;

use crate::bucket::Bucket;

#[derive(Debug)]
pub struct Node<K: Hash, V, const N: u64> {
    pub address: SocketAddr,
    predecessor: Option<SocketAddr>,
    successor: Option<SocketAddr>,

    bucket: Range<Bucket<N>>,
    store: HashMap<K, V>,
}

impl<K, V, const N: u64> Node<K, V, N>
where
    K: Hash,
{
    pub fn new(address: SocketAddr, successor: SocketAddr) -> Self {
        let bucket_id = Bucket::<N>::get_bucket(address);
        let successor_id = Bucket::<N>::get_bucket(successor);
        Node {
            address: address,
            predecessor: None,
            successor: None,
            bucket: bucket_id..successor_id + 1,

            store: HashMap::<K, V>::new(),
        }
    }

    pub fn contains_bucket(&self, b: Bucket<N>) -> bool {
        self.bucket.contains(&b)
    }
}
