use std::ops::{Add, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Identifier(u64);

impl Identifier {}

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
