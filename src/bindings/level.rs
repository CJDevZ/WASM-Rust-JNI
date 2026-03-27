use crate::bindings::protobuf::level_change::BlockUpdate;
use crate::bindings::protobuf::LevelChange;
use crate::bindings::UNIVERSE;
use crate::example::plugin::bindings::{BlockPos, HostLevel};
use crate::plugin::PluginImpl;
use slotmap::{DefaultKey, Key};
use wasmtime::component::Resource;

#[derive(Clone, Copy)]
pub struct LevelHandle(pub u64);

pub struct ShadowLevel {
    uid: DefaultKey,
    pub block_update_queue: Vec<BlockUpdate>,
    pub dirty: u64
}

impl ShadowLevel {
    const BLOCK_UPDATE_DIRTY: u64 = 1 << 0;

    pub fn new(uid: DefaultKey) -> Self {
        Self {
            uid,
            block_update_queue: Vec::new(),
            dirty: 0
        }
    }

    pub fn encode_changes(&mut self) -> Option<LevelChange> {
        if self.dirty == 0 {
            return None
        }

        Some(LevelChange {
            universal_id: self.uid.data().as_ffi(),
            block_updates: std::mem::take(&mut self.block_update_queue)
        })
    }
}

impl HostLevel for PluginImpl {
    fn set_block(&mut self, self_: Resource<LevelHandle>, position: BlockPos, block: String) -> () {
        let level_handle: &LevelHandle = self.table.get(&self_).unwrap();
        UNIVERSE.with_level_mut(level_handle.0, |level| {
            level.block_update_queue.push(BlockUpdate {
                pos: Some(crate::bindings::protobuf::BlockPos {
                    x: position.x,
                    y: position.y,
                    z: position.z
                }),
                block,
            });
            level.dirty |= ShadowLevel::BLOCK_UPDATE_DIRTY
        });
    }

    fn drop(&mut self, rep: Resource<LevelHandle>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}
