
mod api;
mod bindings;

use crate::api::BlockPos;
use crate::bindings::events::{BreakBlockEvent, PlayerEvent};
use crate::example::plugin::{block_registry, level, player};
use std::sync::OnceLock;

wit_bindgen::generate!({
    path: "wit/api14.wit",
    world: "plugin"
});

export!(RustMC);

struct RustMC;
const PLUGIN_ID: &str = "rust-mc";

static GOLD_BLOCK: OnceLock<i32> = OnceLock::new();

impl Guest for RustMC {
    fn plugin_id() -> String {
        String::from(PLUGIN_ID)
    }

    fn entry() -> () {
        use example::plugin::logging;

        logging::info("Hello from Plugin");
    }

    fn handle_event(event_type: EventType, message: Vec<u8>) -> EventResult {
        match event_type {
            api::event::PLAYER_JOIN => {
                let event = PlayerEvent::from_message(message.as_slice());
                let player_handle = event.uid;
                let username = player::get_username(player_handle);
                player::send_message(player_handle, api::text::PLAIN, &format!("Welcome to the server, {} :3", username), false);
            }
            api::event::PLAYER_TICK => {
                let event = PlayerEvent::from_message(message.as_slice());
                let player_handle = event.uid;
                // player::send_message(player, &player::TextComponent::Plain("Ticking player".to_string()), false);
                let mut block_pos = BlockPos::from_u64(player::get_block_pos(player_handle));
                block_pos.y -= 1;
                let gold_block = *GOLD_BLOCK.get_or_init(|| block_registry::get_block_state("minecraft:stripped_cherry_wood", &[]));
                let level_handle = player::get_level(player_handle);
                level::set_block(level_handle, block_pos.to_u64(), gold_block);
                player::custom_command(player_handle, *api::player_commands::SGUI_OPEN_MENU, b"Test Message")
                // player::give_item(player, "diamond", 1);
            }
            api::event::BREAK_BLOCK => {
                let event = BreakBlockEvent::from_message(message.as_slice());
                let player_handle = event.player;
                let block_pos = BlockPos::from_u64(event.block_pos);
                player::send_message(player_handle, api::text::PLAIN, &format!("Broke block at {}, {}, {}", block_pos.x, block_pos.y, block_pos.z), false);
                const PROTECTED_BLOCK: u64 = BlockPos::xyz_to_u64(-33, 75, -62);
                if event.block_pos == PROTECTED_BLOCK {
                    return api::event::RESULT_DENY;
                }
            }
            _ => {}
        };
        api::event::RESULT_DEFAULT
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // let result = sum(2, 2);
        // assert_eq!(result, 4);
    }

    #[test]
    fn totally_normal() {

    }
}
