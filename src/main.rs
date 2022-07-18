#[macro_use]
#[cfg(test)]
extern crate proptest;

mod ibf;
mod strata_estimator;

use std::{collections::HashMap, mem};

use rand::RngCore;

use ibf::*;
use strata_estimator::*;

fn main() {
    // test_recoverability();
    test_strata();
}

const N: usize = 80;

fn test_recoverability() {
    fn test_recoverability_with<const N: usize>(num: usize) -> bool {
        let mut ibf = IBF::<N>::default();
        for rand in random_elems().take(num) {
            ibf.insert(rand);
        }
        (_, ibf) = ibf.recover_items();
        ibf.is_empty()
    }

    fn test_recoverability_probability<const N: usize>(samples: usize, fill_num: usize) -> f64 {
        let mut count = 0;
        for _ in 0..samples {
            if test_recoverability_with::<N>(fill_num) {
                count += 1;
            }
        }
        (count as f64) / (samples as f64)
    }

    let jump_size = 2;
    let begin = 0;
    let end = 80;
    for i in (begin / jump_size)..(end / jump_size) {
        let fill_num = i * jump_size;
        let prob = test_recoverability_probability::<N>(2000500, fill_num);
        println!("Filter of size {N} with {fill_num} elements recovers with probability {prob}");
    }
}

struct RandomElems(blake3::OutputReader);

fn random_elems() -> impl Iterator<Item = [u8; HASH_SIZE]> {
    RandomElems(blake3::Hasher::new().update(&random_elem()).finalize_xof())
}

impl Iterator for RandomElems {
    type Item = [u8; HASH_SIZE];

    fn next(&mut self) -> Option<Self::Item> {
        let mut res = [0u8; HASH_SIZE];
        self.0.fill(&mut res);
        Some(res)
    }
}

fn random_elem() -> [u8; HASH_SIZE] {
    let mut randoms = [0u8; HASH_SIZE];
    rand::thread_rng().fill_bytes(&mut randoms);
    randoms
}

fn test_strata() {
    const S: usize = 16;
    let mut estimator1 = Estimator::<S>::default();
    let mut estimator2 = Estimator::<S>::default();
    let mut difference = Vec::new();

    let item_count = 1_000_000;
    let diff_count = 100;

    for i in 0..item_count {
        let item = format!("Hello, for the {i}th time!");
        let in_left = i > diff_count;
        let in_right = i < item_count - diff_count;
        if in_left {
            estimator1.insert(&item);
        }
        if in_right {
            estimator2.insert(&item);
        }
        if !in_left || !in_right {
            difference.push(item);
        }
    }

    let mut diff_set = HashMap::<[u8; HASH_SIZE], ()>::default();

    for item in difference.iter() {
        diff_set.insert(*blake3::hash(item.as_ref()).as_bytes(), ());
    }

    let estimated = (estimator1 - estimator2).estimate();
    println!("Estimated diff: {estimated}, actual: {}", difference.len());
}

fn test_ibfs() {
    const N: usize = 1000;
    let mut ibf: IBF<N> = IBF::default();
    let mut ibf2: IBF<N> = IBF::default();
    let mut difference = Vec::new();

    let item_count = 10_000_000;
    let diff_count = 380;

    for i in 0..item_count {
        let item = format!("Hello, for the {i}th time!");
        let in_left = i > diff_count;
        let in_right = i < item_count - diff_count;
        if in_left {
            ibf.insert(&item);
        }
        if in_right {
            ibf2.insert(&item);
        }
        if !in_left || !in_right {
            difference.push(item);
        }
    }

    let mut diff_set = HashMap::<[u8; HASH_SIZE], ()>::default();

    for item in difference.iter() {
        diff_set.insert(*blake3::hash(item.as_ref()).as_bytes(), ());
    }

    let mut diff_iter = (ibf - ibf2).recover();
    let mut count = 0;

    while let Some(item) = diff_iter.next() {
        if diff_set.contains_key(item.get_hash()) {
            count += 1;
        }
    }

    println!(
        "{}/{}. Resolved? {}. {} bytes.",
        count,
        difference.len(),
        diff_iter.is_fully_recovered(),
        mem::size_of::<IBF<N>>()
    );
}
