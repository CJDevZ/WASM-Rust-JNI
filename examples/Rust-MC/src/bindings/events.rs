use crate::bindings::{get_i32, get_u64};
use crate::example::plugin::level::BlockPos;
use crate::example::plugin::player::{LevelHandle, PlayerHandle};

pub struct PlayerEvent {
    pub uid: PlayerHandle,
}

impl PlayerEvent {
    pub fn from_message(message: &[u8]) -> PlayerEvent {
        PlayerEvent {
            uid: get_u64(message, 0),
        }
    }
}

pub struct BreakBlockEvent {
    pub player: PlayerHandle,
    pub level: LevelHandle,
    pub block_pos: BlockPos,
    pub block_state: i32,
}

impl BreakBlockEvent {
    pub fn from_message(message: &[u8]) -> BreakBlockEvent {
        BreakBlockEvent {
            player: get_u64(message, 0),
            level: get_u64(message, 8),
            block_pos: get_u64(message, 16),
            block_state: get_i32(message, 24),
        }
    }
}
