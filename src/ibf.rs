use std::ops::{Add, AddAssign, Sub, SubAssign};

use xxhash_rust::xxh3::{xxh3_64, xxh3_64_with_seed};

pub const HASH_SIZE: usize = 32;

#[derive(Debug, Clone, Copy)]
pub struct IBF<const N: usize, const K: usize = 4> {
    pub cells: [Cell; N],
}

#[derive(Debug, Clone, Copy)]
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

fn map_rand_to_range(rand: u64, range: u64) -> u64 {
    let last_32 = rand & 0xFFFF_FFFF;
    let first_32 = rand >> 32;
    let trunc = last_32 ^ first_32;
    trunc * range >> 32
}

impl<const N: usize, const K: usize> IBF<N, K> {
    pub fn insert<A: AsRef<[u8]>>(&mut self, item: A) {
        self.insert_hash(blake3::hash(item.as_ref()).as_bytes());
    }

    pub fn remove<A: AsRef<[u8]>>(&mut self, item: A) {
        self.remove_hash(blake3::hash(item.as_ref()).as_bytes());
    }

    pub fn insert_hash(&mut self, item_hash: &[u8; HASH_SIZE]) {
        for seed in 0..K {
            let idx = map_rand_to_range(xxh3_64_with_seed(item_hash, seed as u64), N as u64);
            self.cells[idx as usize] += Cell::new(*item_hash);
        }
    }

    pub fn remove_hash(&mut self, item_hash: &[u8; HASH_SIZE]) {
        for seed in 0..K {
            let idx = map_rand_to_range(xxh3_64_with_seed(item_hash, seed as u64), N as u64);
            self.cells[idx as usize] -= Cell::new(*item_hash);
        }
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

    pub fn recover_items(self) -> RecoverIterator<N, K> {
        RecoverIterator { filter: self }
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
