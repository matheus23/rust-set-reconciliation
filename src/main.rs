#[macro_use]
extern crate proptest;

mod ibf;
mod strata_estimator;

use std::{collections::HashMap, mem};

use ibf::*;
use strata_estimator::*;

fn main() {
    test_strata();
}

fn test_strata() {
    const S: usize = 16;
    let mut estimator1 = Estimator::<S>::default();
    let mut estimator2 = Estimator::<S>::default();
    let mut difference = Vec::new();

    let item_count = 10_000_000;
    let diff_count = 100_000;

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
