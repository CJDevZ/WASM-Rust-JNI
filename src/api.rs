
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn from_long(packed: u64) -> BlockPos {
        BlockPos {
            x: (packed >> 38) as i32,
            y: (packed & 0xFFF) as i32,
            z: ((packed >> 12) & 0x3FFFFFF) as i32,
        }
    }

    pub fn to_long(&self) -> crate::example::plugin::level::BlockPos {
        Self::xyz_to_long(self.x, self.y, self.z)
    }

    #[inline(always)]
    pub fn xyz_to_long(x: i32, y: i32, z: i32) -> crate::example::plugin::level::BlockPos {
        (((x as u64 & 0x3FFFFFF) << 38) | ((z as u64 & 0x3FFFFFF) << 12) | y as u64 & 0xFFF) as crate::example::plugin::level::BlockPos
    }
}

pub mod text {
    use crate::example::plugin::bindings::TextType;

    const PLAIN: TextType = 0;
    const MINI_MESSAGE: TextType = 1;
    const JSON: TextType = 2;
}

pub mod event {
    use crate::exports::example::plugin::events::{EventResult, EventType};

    pub const PLAYER_JOIN: EventType = 1;
    pub const PLAYER_LEAVE: EventType = 2;
    pub const TICK: EventType = 3;
    pub const PLAYER_TICK: EventType = 4;
    pub const BREAK_BLOCK: EventType = 5;
    pub const PLACE_BLOCK: EventType = 6;

    pub const RESULT_DEFAULT: EventResult = 0;
    pub const RESULT_ALLOW: EventResult = 1;
    pub const RESULT_DENY: EventResult = 2;
}
