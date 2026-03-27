use crate::example;
use crate::plugin::PluginImpl;
use crate::universe::Universe;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod player;
pub mod logger;
pub mod level;

impl example::plugin::bindings::Host for PluginImpl {}

pub static UNIVERSE: Lazy<Universe> = Lazy::new(Universe::new);

pub mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.rs"));
}

pub struct AtomicBitSet {
    buckets: Vec<AtomicU64>,
}

impl AtomicBitSet {
    pub fn new(capacity_bits: usize) -> Self {
        let mut buckets = Vec::with_capacity((capacity_bits + 63) / 64);
        buckets.fill_with(|| AtomicU64::new(0));
        AtomicBitSet {
            buckets,
        }
    }

    pub fn mark_dirty(&self, index: usize) {
        let bucket_idx = index / 64;
        let bit_idx = index % 64;
        let mask = 1 << bit_idx;

        self.buckets[bucket_idx].fetch_or(mask, Ordering::Relaxed);
    }

    pub fn get_and_clear(&self, bucket_idx: usize) -> u64 {
        self.buckets[bucket_idx].swap(0, Ordering::Acquire)
    }

    pub fn grow(&mut self, num_bits: usize) {
        self.buckets.resize_with((num_bits + 63) / 64, || AtomicU64::new(0));
    }
}
