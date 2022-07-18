use std::{
    cmp,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use crate::ibf::{HASH_SIZE, IBF};

const N: usize = 80;

#[derive(Debug, Clone, Copy)]
pub struct Estimator<const S: usize = 16> {
    pub strata: [IBF<N>; S],
}

impl<const S: usize> Default for Estimator<S> {
    fn default() -> Self {
        Self {
            strata: [IBF::<N>::default(); S],
        }
    }
}

impl<const S: usize> Estimator<S> {
    fn bucket_for_hash(item_hash: &[u8; HASH_SIZE]) -> usize {
        cmp::min(S - 1, leading_zeros(item_hash) as usize)
    }

    pub fn insert<A: AsRef<[u8]>>(&mut self, item: A) {
        self.insert_hash(blake3::hash(item.as_ref()).as_bytes());
    }

    pub fn insert_hash(&mut self, item_hash: &[u8; HASH_SIZE]) {
        self.strata[Self::bucket_for_hash(item_hash)].insert_hash(item_hash)
    }

    pub fn remove<A: AsRef<[u8]>>(&mut self, item: A) {
        self.remove_hash(blake3::hash(item.as_ref()).as_bytes());
    }

    pub fn remove_hash(&mut self, item_hash: &[u8; HASH_SIZE]) {
        self.strata[Self::bucket_for_hash(item_hash)].remove_hash(item_hash)
    }

    pub fn estimate(&self) -> u64 {
        let mut count = 0;

        for level in (-1..S as i64).rev() {
            if level < 0 {
                break;
            }

            let ibf = self.strata[level as usize];
            let mut iter = ibf.recover();
            let mut recovered = 0;
            while let Some(_) = iter.next() {
                recovered += 1;
            }
            let ok = iter.is_fully_recovered();

            if !ok {
                return (2 << level) * count + (1 << level);
            }

            count += recovered;
        }

        return count;
    }
}

impl<const S: usize> AddAssign for Estimator<S> {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..S {
            self.strata[i] += rhs.strata[i];
        }
    }
}

impl<const S: usize> Add for Estimator<S> {
    type Output = Estimator<S>;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl<const S: usize> SubAssign for Estimator<S> {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..S {
            self.strata[i] -= rhs.strata[i];
        }
    }
}

impl<const S: usize> Sub for Estimator<S> {
    type Output = Estimator<S>;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

fn leading_zeros(item_hash: &[u8; HASH_SIZE]) -> u32 {
    let mut i = 0;
    let mut total = 0;
    loop {
        let zeros = item_hash[i].leading_zeros();
        total += zeros;
        i += 1;
        if zeros != 8 || i >= HASH_SIZE {
            return total;
        }
    }
}

#[cfg(test)]
mod strata_estimator_tests {
    use hex::FromHexError;

    use crate::ibf::HASH_SIZE;

    use super::leading_zeros;

    #[test]
    fn test_leading_zeros_256() {
        assert_eq!(leading_zeros(&[0u8; HASH_SIZE]), 256)
    }

    #[test]
    fn test_leading_zeros_128() {
        let zeros_hash = parse_hash_hex(
            "00000000000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".into(),
        )
        .unwrap();
        assert_eq!(leading_zeros(&zeros_hash), 128)
    }

    #[test]
    fn test_leading_zeros_12() {
        let zeros_hash = parse_hash_hex(
            "000F0000000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".into(),
        )
        .unwrap();
        assert_eq!(leading_zeros(&zeros_hash), 12)
    }

    fn parse_hash_hex(hex: String) -> Result<[u8; HASH_SIZE], FromHexError> {
        let mut hash = [0u8; HASH_SIZE];
        hex::decode_to_slice(hex, &mut hash)?;
        Ok(hash)
    }
}
