use std::{
    iter,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use blake3::Hash;
use xxhash_rust::xxh3::{xxh3_64, xxh3_64_with_seed};

pub const HASH_SIZE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IBF<const N: usize, const K: usize = 4> {
    pub cells: [Cell; N],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub id: [u8; HASH_SIZE],
    pub hash: u64,
    pub count: i64,
}

#[derive(Debug)]
pub enum PureCell {
    Pos([u8; HASH_SIZE]),
    Neg([u8; HASH_SIZE]),
}

impl PureCell {
    pub fn get_hash(&self) -> &[u8; HASH_SIZE] {
        match self {
            Self::Pos(hash) => hash,
            Self::Neg(hash) => hash,
        }
    }
}

impl Cell {
    pub fn new(id: [u8; HASH_SIZE]) -> Self {
        let hash = xxh3_64(&id);
        Self { id, hash, count: 1 }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn get_if_pure(&self) -> Option<PureCell> {
        match self.count {
            -1 => {
                if self.hash_matches() {
                    Some(PureCell::Neg(self.id))
                } else {
                    None
                }
            }
            1 => {
                if self.hash_matches() {
                    Some(PureCell::Pos(self.id))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn hash_matches(&self) -> bool {
        xxh3_64(&self.id) == self.hash
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            id: [0u8; HASH_SIZE],
            hash: 0,
            count: 0,
        }
    }
}

impl Add for Cell {
    type Output = Cell;

    fn add(mut self, rhs: Cell) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Cell {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..HASH_SIZE {
            self.id[i] = self.id[i] ^ rhs.id[i];
        }
        self.hash ^= rhs.hash;
        self.count += rhs.count;
    }
}

impl Sub for Cell {
    type Output = Cell;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

impl SubAssign for Cell {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..HASH_SIZE {
            self.id[i] = self.id[i] ^ rhs.id[i];
        }
        self.hash ^= rhs.hash;
        self.count -= rhs.count;
    }
}

// TODO(matheus23) remove. This is unused.
fn map_rand_to_range(rand: u64, range: u64) -> u64 {
    let last_32 = rand & 0xFFFF_FFFF;
    let first_32 = rand >> 32;
    let trunc = last_32 ^ first_32;
    trunc * range >> 32
}

pub fn distinct_hashes_in_range<const N: usize, const K: usize>(
    item_hash: &[u8],
) -> impl Iterator<Item = usize> + '_ {
    let mut used_nums = [false; N];

    let bitmask = (if N.count_ones() == 1 {
        N
    } else {
        N.next_power_of_two()
    } - 1);

    let mut seed = 0;
    let mut count = 0;

    iter::from_fn(move || {
        if count >= K {
            return None;
        }

        let mut hash = (xxh3_64_with_seed(item_hash, seed) as usize) & bitmask;

        loop {
            // Try to generate something within bounds
            while hash >= N {
                seed += 1;
                hash = (xxh3_64_with_seed(item_hash, seed) as usize) & bitmask;
            }

            // If it has already been generated, try again
            if !used_nums[hash] {
                break;
            }
            seed += 1;
            hash = (xxh3_64_with_seed(item_hash, seed) as usize) & bitmask;
        }

        used_nums[hash] = true;
        count += 1;
        Some(hash)
    })
}

impl<const N: usize, const K: usize> IBF<N, K> {
    pub fn insert<A: AsRef<[u8]>>(&mut self, item: A) {
        self.insert_hash(blake3::hash(item.as_ref()).as_bytes());
    }

    pub fn remove<A: AsRef<[u8]>>(&mut self, item: A) {
        self.remove_hash(blake3::hash(item.as_ref()).as_bytes());
    }

    pub fn insert_hash(&mut self, item_hash: &[u8; HASH_SIZE]) {
        for idx in distinct_hashes_in_range::<N, K>(item_hash) {
            self.cells[idx as usize] += Cell::new(*item_hash);
        }
    }

    pub fn remove_hash(&mut self, item_hash: &[u8; HASH_SIZE]) {
        for idx in distinct_hashes_in_range::<N, K>(item_hash) {
            self.cells[idx] -= Cell::new(*item_hash);
        }
    }

    pub fn insert_blake3(&mut self, hash: &Hash) {
        self.insert_hash(hash.as_bytes());
    }

    pub fn remove_blake3(&mut self, hash: &Hash) {
        self.remove_hash(hash.as_bytes());
    }

    pub fn find_pure(&self) -> Option<PureCell> {
        for cell in self.cells.iter() {
            if let Some(pure_cell) = cell.get_if_pure() {
                return Some(pure_cell);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        for cell in self.cells.iter() {
            if !cell.is_empty() {
                return false;
            }
        }
        true
    }

    pub fn recover(self) -> RecoverIterator<N, K> {
        RecoverIterator { filter: self }
    }

    pub fn recover_items(self) -> (Vec<PureCell>, Self) {
        let mut iter = self.recover();
        let mut vec = Vec::new();
        while let Some(item) = iter.next() {
            vec.push(item);
        }
        (vec, iter.filter)
    }
}

impl<const N: usize, const K: usize> Add<IBF<N, K>> for IBF<N, K> {
    type Output = IBF<N, K>;

    fn add(mut self, rhs: IBF<N, K>) -> Self::Output {
        self += rhs;
        self
    }
}

impl<const N: usize, const K: usize> AddAssign for IBF<N, K> {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..N {
            self.cells[i] += rhs.cells[i];
        }
    }
}

impl<const N: usize, const K: usize> Sub<IBF<N, K>> for IBF<N, K> {
    type Output = IBF<N, K>;

    fn sub(mut self, rhs: IBF<N, K>) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<const N: usize, const K: usize> SubAssign for IBF<N, K> {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..N {
            self.cells[i] -= rhs.cells[i];
        }
    }
}

pub struct RecoverIterator<const N: usize, const K: usize> {
    pub filter: IBF<N, K>,
}

impl<const N: usize, const K: usize> RecoverIterator<N, K> {
    pub fn is_fully_recovered(&self) -> bool {
        self.filter.is_empty()
    }
}

impl<const N: usize, const K: usize> Iterator for RecoverIterator<N, K> {
    type Item = PureCell;

    fn next(&mut self) -> Option<Self::Item> {
        self.filter.find_pure().map(|pure| {
            match &pure {
                PureCell::Pos(hash) => self.filter.remove_hash(hash),
                PureCell::Neg(hash) => self.filter.insert_hash(hash),
            }
            pure
        })
    }
}

impl<const N: usize, const K: usize> Default for IBF<N, K> {
    fn default() -> Self {
        Self {
            cells: [Cell::default(); N],
        }
    }
}

#[cfg(test)]
mod ibf_tests {
    use std::collections::HashSet;

    use super::distinct_hashes_in_range;
    use super::map_rand_to_range;
    use super::IBF;
    use blake3::Hash;
    use proptest::tuple;
    use proptest::{collection::hash_set, prelude::*};
    use xxhash_rust::xxh3::xxh3_64;

    fn hashes(max_num: usize) -> impl Strategy<Value = HashSet<Hash>> {
        hash_set(any::<String>(), 0..max_num).prop_map(|set| {
            set.iter()
                .map(|elem| blake3::hash(elem.as_bytes()))
                .collect()
        })
    }

    fn ibf_filled_up_to_with_size<const N: usize>(
        max_elems: usize,
    ) -> impl Strategy<Value = (usize, IBF<N>)> {
        hash_set(any::<String>(), 0..max_elems).prop_map(|set| {
            let mut ibf: IBF<N> = IBF::default();
            for elem in set.iter() {
                ibf.insert(elem);
            }
            (set.len(), ibf)
        })
    }

    fn recoverable_ibf<const N: usize>() -> impl Strategy<Value = (usize, IBF<N>)> {
        ibf_filled_up_to_with_size::<N>(N / 2)
    }

    fn ibf_filled_up_to<const N: usize>(max_elems: usize) -> impl Strategy<Value = IBF<N>> {
        ibf_filled_up_to_with_size::<N>(max_elems).prop_map(|(_, ibf)| ibf)
    }

    proptest! {
        #[test]
        fn sub_itself_is_zero(ibf in ibf_filled_up_to::<80>(100)) {
            assert!((ibf - ibf).is_empty())
        }

        #[test]
        fn sub_is_add_inverse(ibf in ibf_filled_up_to::<80>(100)) {
            assert!((ibf + (IBF::default() - ibf)).is_empty())
        }

        #[test]
        fn add_is_associative((a, b, c) in (ibf_filled_up_to::<80>(100), ibf_filled_up_to::<80>(100), ibf_filled_up_to::<80>(100))) {
            assert_eq!(((a + b) + c), (a + (b + c)))
        }

        #[test]
        fn add_is_commutative((a, b) in (ibf_filled_up_to::<80>(100), ibf_filled_up_to::<80>(100))) {
            assert_eq!((a + b), (b + a))
        }

        #[test]
        fn ibf_recovers((elems, ibf) in recoverable_ibf::<50>()) {
            let mut iter = ibf.recover();
            let mut count = 0;
            while let Some(_) = iter.next() {
                count += 1;
            }
            assert!(iter.is_fully_recovered());
            assert_eq!(count, elems);
        }

        #[test]
        fn ibf_recovers2(hs in hashes(40)) {
            let mut ibf: IBF<80> = IBF::default();
            for hash in hs.iter() {
                ibf.insert_blake3(hash);
            }

            let mut iter = ibf.recover();
            let mut count = 0;
            while let Some(_) = iter.next() {
                count += 1;
            }
            assert!(iter.is_fully_recovered());
            assert_eq!(count, hs.len());
        }

        #[test]
        fn distinct_hashing(s in any::<String>()) {
            const R: usize = 10;
            let mut bits = [false; R];
            for i in distinct_hashes_in_range::<R, 4>(s.as_bytes()) {
                if bits[i] {
                    panic!("Generated {i} twice!");
                }
                bits[i] = true;
            }
        }

        #[test]
        fn test_map_rand_to_range((elem, max) in (any::<String>(), 10u64..1000)) {
            let value = map_rand_to_range(xxh3_64(elem.as_bytes()), max);
            assert!(value < max)
        }
    }
}
