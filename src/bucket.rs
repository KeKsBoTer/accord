use serde::{Deserialize, Serialize};

use sha1::{Digest, Sha1};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::ops;

#[derive(Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Bucket<const N: u64>(u64);

impl<const N: u64> From<u64> for Bucket<N> {
    fn from(n: u64) -> Self {
        Bucket::<N>(n % N)
    }
}

impl<const N: u64> ops::Add<u64> for Bucket<N> {
    type Output = Bucket<N>;

    fn add(self, _rhs: u64) -> Bucket<N> {
        Bucket::from(self.0 + _rhs)
    }
}

// TODO maybe implement own Consistent Hash trait?

impl<const N: u64> Bucket<N> {
    pub fn get_bucket(data: impl Hash) -> Bucket<N> {
        let mut hasher = ConsistentHasher::<N>(Sha1::new());
        // match addr.ip() {
        //     IpAddr::V4(d) => hasher.update(d.octets()),
        //     IpAddr::V6(d) => hasher.update(d.octets()),
        // };
        // hasher.update(addr.port().to_le_bytes());
        data.hash(&mut hasher);
        return Bucket::from(hasher.finish());
    }
}
struct ConsistentHasher<const N: u64>(Sha1);

impl<const N: u64> Hasher for ConsistentHasher<N> {
    fn finish(&self) -> u64 {
        // let result = self.0.finalize_fixed();
        // let id = BigUint::from_bytes_le(&result) % N;
        // return 0 *id.to_u64_digits().first().unwrap();
        todo!("oh no");
        return 0;
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.write(&bytes).unwrap();
    }
}
