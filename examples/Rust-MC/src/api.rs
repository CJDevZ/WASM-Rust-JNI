
#[derive(Copy, Clone)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn from_u64(packed: u64) -> BlockPos {
        BlockPos {
            x: ((packed >> 32) as i32) >> 6,
            y: ((packed << 20) as i32) >> 20,
            z: ((packed >> 6) as i32) >> 6,
        }
    }

    pub fn to_u64(self) -> crate::example::plugin::level::BlockPos {
        Self::xyz_to_u64(self.x, self.y, self.z)
    }

    pub const fn xyz_to_u64(x: i32, y: i32, z: i32) -> crate::example::plugin::level::BlockPos {
        (
            (((x & 0x3FFFFFF) as u64) << 38)
            | (((z & 0x3FFFFFF) as u64) << 12)
            | (y & 0xFFF) as u64
        ) as crate::example::plugin::level::BlockPos
    }
}

pub mod text {
    use crate::example::plugin::bindings::TextType;

    pub const PLAIN: TextType = 0;
    pub const MINI_MESSAGE: TextType = 1;
    pub const JSON: TextType = 2;
}

pub mod event {
    use once_cell::unsync::Lazy;
    use crate::{EventResult, EventType};

    pub const PLAYER_JOIN: EventType = 1;
    pub const PLAYER_LEAVE: EventType = 2;
    pub const TICK: EventType = 3;
    pub const PLAYER_TICK: EventType = 4;
    pub const BREAK_BLOCK: EventType = 5;
    pub const PLACE_BLOCK: EventType = 6;
    pub const CUSTOM_TEST_EVENT: Lazy<EventType> = Lazy::new(|| crate::example::plugin::events::get_event_id("custom_test_event"));

    pub const RESULT_DEFAULT: EventResult = 0;
    pub const RESULT_ALLOW: EventResult = 1;
    pub const RESULT_DENY: EventResult = -1;
}

pub mod player_commands {
    use once_cell::sync::Lazy;

    pub const SGUI_OPEN_MENU: Lazy<i32> = Lazy::new(|| crate::player::get_custom_command_id("sgui:open_menu"));
}
