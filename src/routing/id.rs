use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Debug, Display, Formatter};
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};
use std::ops::{Add, Sub};
#[derive(Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Identifier(u64);

impl Identifier {
    // Returns whether this identifier is between `start` (exclusive) and `end` (inclusive) on the
    // identifier ring
    pub fn is_between(&self, start: Identifier, end: Identifier) -> bool {
        if start == end {
            return true;
        }
        if *self > start && end >= *self {
            return true;
        }

        if end >= *self && start > end {
            return true;
        }

        if start > end && *self > start {
            return true;
        }
        return false;
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl From<u64> for Identifier {
    fn from(value: u64) -> Self {
        Identifier(value)
    }
}

impl From<Identifier> for u64 {
    fn from(value: Identifier) -> Self {
        value.0
    }
}

impl From<&[u8]> for Identifier {
    fn from(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let id = BigUint::from_bytes_le(digest.as_slice());
        let ring = id % std::u64::MAX;
        Identifier(*ring.to_u64_digits().first().unwrap() as u64)
    }
}

impl Add for Identifier {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let (sum, _) = self.0.overflowing_add(other.0);

        Identifier(sum)
    }
}

impl Sub for Identifier {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let (diff, _) = self.0.overflowing_sub(other.0);

        Identifier(diff)
    }
}

impl Debug for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        return f.write_fmt(format_args!("{:}", self.0 / 281474976710656));
    }
}

pub trait HashIdentifier<T> {
    fn hash_id(&self) -> T;
}

impl HashIdentifier<Identifier> for SocketAddrV4 {
    fn hash_id(&self) -> Identifier {
        let mut bytes = [0u8; 6];
        bytes[..4].copy_from_slice(&self.ip().octets());
        bytes[4..].copy_from_slice(&self.port().to_le_bytes());
        Identifier::from(bytes.as_ref())
    }
}

impl HashIdentifier<Identifier> for SocketAddrV6 {
    fn hash_id(&self) -> Identifier {
        let mut bytes = [0u8; 18];
        bytes[..16].copy_from_slice(&self.ip().octets());
        bytes[16..].copy_from_slice(&self.port().to_le_bytes());
        Identifier::from(bytes.as_ref())
    }
}

impl HashIdentifier<Identifier> for SocketAddr {
    fn hash_id(&self) -> Identifier {
        match self {
            SocketAddr::V4(addr) => addr.hash_id(),
            SocketAddr::V6(addr) => addr.hash_id(),
        }
    }
}

impl HashIdentifier<Identifier> for String {
    fn hash_id(&self) -> Identifier {
        Identifier::from(self.as_bytes().as_ref())
    }
}
