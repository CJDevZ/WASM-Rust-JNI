use crate::bindings::protobuf::core::player::player_command::Action;
use crate::bindings::protobuf::core::player::{GiveItemCommand, PlayerChange, PlayerCommand, SendMessageCommand};
use crate::bindings::{protobuf, Syncable, UNIVERSE};
use crate::{api, example};
use crate::example::plugin::bindings::TextType;
use crate::example::plugin::level::LevelHandle;
use crate::example::plugin::player::BlockPos;
use crate::plugin::PluginImpl;

pub struct ShadowPlayer {
    level_handle: LevelHandle,
    position: example::plugin::bindings::Vec3,
    pub command_queue: Vec<PlayerCommand>,
    pub dirty: u64,
}

impl Syncable for ShadowPlayer {
    type Change = PlayerChange;

    fn encode_changes(&mut self, universal_id: u64) -> Option<PlayerChange> {
        (self.dirty != 0).then(|| {
            let position = (self.dirty & ShadowPlayer::POSITION_DIRTY != 0).then(|| {
                protobuf::core::Vec3 {
                    x: self.position.x,
                    y: self.position.y,
                    z: self.position.z
                }
            });

            self.dirty = 0;
            PlayerChange {
                universal_id,
                position,
                commands: std::mem::take(&mut self.command_queue),
            }
        })
    }

    fn decode_changes(&mut self, change: Self::Change) {
        if let Some(position) = change.position {
            self.position = example::plugin::bindings::Vec3 {x: position.x, y: position.y, z: position.z };
        }
    }
}

impl ShadowPlayer {
    const POSITION_DIRTY: u64 = 1 << 0;
    const COMMAND_QUEUE_DIRTY: u64 = 1 << 1;

    pub fn new(level_handle: LevelHandle) -> Self {
        Self {
            level_handle,
            position: example::plugin::bindings::Vec3 {x: 0f64, y: 0f64, z: 0f64},
            command_queue: Vec::new(),
            dirty: 0,
        }
    }
}

impl example::plugin::player::Host for PluginImpl {
    fn get_level(&mut self, player: example::plugin::player::PlayerHandle) -> example::plugin::player::LevelHandle {
        UNIVERSE.players.with(player, |player| {
            player.level_handle
        }).expect("Couldn't get player")
    }

    fn get_block_pos(&mut self, player: example::plugin::player::PlayerHandle) -> BlockPos {
        UNIVERSE.players.with(player, |player| {
            api::BlockPos::xyz_to_long(
                player.position.x.floor() as i32,
                player.position.y.floor() as i32,
                player.position.z.floor() as i32,
            )
        }).expect("Couldn't get player")
    }

    fn send_message(&mut self, player: example::plugin::player::PlayerHandle, message_type: TextType, message: String, action_bar: bool) -> () {
        let text = protobuf::core::player::TextComponent {
            message_type,
            message: message.clone()
        };
        UNIVERSE.players.with_mut(player, |player| {
            player.command_queue.push(PlayerCommand {
                action: Some(Action::SendMessage(SendMessageCommand {
                    text: Some(text),
                    action_bar
                }))
            });
            player.dirty |= ShadowPlayer::COMMAND_QUEUE_DIRTY;
        });
    }

    fn teleport(&mut self, player: example::plugin::player::PlayerHandle, x: f64, y: f64, z: f64) -> () {
        UNIVERSE.players.with_mut(player, |player| {
            player.position = example::plugin::bindings::Vec3 { x, y, z };
            player.dirty |= ShadowPlayer::POSITION_DIRTY;
        });
    }

    fn give_item(&mut self, player: example::plugin::player::PlayerHandle, item: String, count: i32) -> () {
        UNIVERSE.players.with_mut(player, |player| {
            player.command_queue.push(PlayerCommand {
                action: Some(Action::GiveItem(GiveItemCommand {
                    item,
                    count
                }))
            });
            player.dirty |= ShadowPlayer::COMMAND_QUEUE_DIRTY;
        });
    }
}
