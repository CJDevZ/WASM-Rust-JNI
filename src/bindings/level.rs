use crate::bindings::protobuf::core::{BlockUpdate, LevelChange};
use crate::bindings::{Syncable, UNIVERSE};
use crate::example::plugin::level::BlockPos;
use crate::get_logger;
use crate::plugin::PluginImpl;

pub struct ShadowLevel {
    pub block_update_queue: Vec<BlockUpdate>,
    pub dirty: u64
}

impl Syncable for ShadowLevel {
    type Change = LevelChange;

    fn encode_changes(&mut self, universal_id: u64) -> Option<LevelChange> {
        (self.dirty != 0).then(|| {
            self.dirty = 0;
            LevelChange {
                universal_id,
                block_updates: std::mem::take(&mut self.block_update_queue)
            }
        })
    }

    fn decode_changes(&mut self, change: Self::Change) {
        todo!()
    }
}

impl ShadowLevel {
    const BLOCK_UPDATE_DIRTY: u64 = 1 << 0;

    pub fn new() -> Self {
        Self {
            block_update_queue: Vec::new(),
            dirty: 0
        }
    }
}

impl crate::example::plugin::level::Host for PluginImpl {
    fn set_block(&mut self, level: crate::example::plugin::level::LevelHandle, position: BlockPos, block: i32) -> () {
        if block < 0 {
            return;
        }
        UNIVERSE.levels.with_mut(level, |level| {
            level.block_update_queue.push(BlockUpdate {
                position,
                block,
            });
            level.dirty |= ShadowLevel::BLOCK_UPDATE_DIRTY
        });
    }
}
