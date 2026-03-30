use crate::bindings::level::LevelHandle;
use crate::bindings::protobuf::player::player_command::Action;
use crate::bindings::protobuf::player::{GiveItemCommand, PlayerChange, PlayerCommand, SendMessageCommand};
use crate::bindings::{protobuf, UNIVERSE};
use crate::example;
use crate::example::plugin::bindings::{BlockPos, TextComponent, Vec3};
use crate::plugin::PluginImpl;
use wasmtime::component::Resource;

#[derive(Clone, Copy)]
pub struct PlayerHandle(pub u64);

pub struct ShadowPlayer {
    level_handle: LevelHandle,
    position: Vec3,
    pub command_queue: Vec<PlayerCommand>,
    pub dirty: u64,
}

impl ShadowPlayer {
    const POSITION_DIRTY: u64 = 1 << 0;
    const COMMAND_QUEUE_DIRTY: u64 = 1 << 1;

    pub fn new(level_handle: LevelHandle) -> Self {
        Self {
            level_handle,
            position: Vec3 {x: 0f64, y: 0f64, z: 0f64},
            command_queue: Vec::new(),
            dirty: 0,
        }
    }

    pub fn decode_changes(&mut self, change: PlayerChange) {
        if let Some(position) = change.position {
            self.position = Vec3 {x: position.x, y: position.y, z: position.z };
        }
    }

    pub fn encode_changes(&mut self, universal_id: u64) -> Option<PlayerChange> {
        (self.dirty != 0).then(|| {
            let position = (self.dirty & ShadowPlayer::POSITION_DIRTY != 0).then(|| {
                protobuf::core::Vec3 {
                    x: self.position.x,
                    y: self.position.y,
                    z: self.position.z
                }
            });

            PlayerChange {
                universal_id,
                position,
                commands: std::mem::take(&mut self.command_queue),
            }
        })
    }
}

impl example::plugin::bindings::HostPlayer for PluginImpl {
    fn get_level(&mut self, self_: Resource<PlayerHandle>) -> Resource<LevelHandle> {
        let player_handle: &PlayerHandle = self.table.get(&self_).unwrap();
        UNIVERSE.with_player(player_handle.0, |player| {
            self.table.push(player.level_handle).unwrap()
        }).unwrap()
    }

    fn get_block_pos(&mut self, self_: Resource<PlayerHandle>) -> BlockPos {
        let player_handle: &PlayerHandle = self.table.get(&self_).unwrap();
        UNIVERSE.with_player(player_handle.0, |player| {
            BlockPos {
                x: player.position.x.floor() as i32,
                y: player.position.y.floor() as i32,
                z: player.position.z.floor() as i32
            }
        }).unwrap()
    }

    fn send_message(&mut self, self_: Resource<PlayerHandle>, msg: TextComponent, action_bar: bool) -> () {
        let player_handle: &PlayerHandle = self.table.get(&self_).unwrap();
        let (text_type, text_message) = match msg {
            TextComponent::Plain(text) => (protobuf::player::text_component::Type::Plain, text),
            TextComponent::MiniMessage(text) => (protobuf::player::text_component::Type::Plain, text),
            TextComponent::Json(text) => (protobuf::player::text_component::Type::Plain, text)
        };
        let text = protobuf::player::TextComponent {
            r#type: text_type as i32,
            message: text_message.clone()
        };
        UNIVERSE.with_player_mut(player_handle.0, |player| {
            player.command_queue.push(PlayerCommand {
                action: Some(Action::SendMessage(SendMessageCommand {
                    text: Some(text),
                    action_bar
                }))
            });
            player.dirty |= ShadowPlayer::COMMAND_QUEUE_DIRTY;
        });
    }

    fn teleport(&mut self, self_: Resource<PlayerHandle>, x: f64, y: f64, z: f64) -> () {
        if let Ok(player_handle) = self.table.get_mut(&self_) {
            UNIVERSE.with_player_mut(player_handle.0, |player| {
                player.position = Vec3 { x, y, z };
                player.dirty |= ShadowPlayer::POSITION_DIRTY;
            });
        }
    }

    fn give_item(&mut self, self_: Resource<PlayerHandle>, item: String, count: i32) -> () {
        if let Ok(player_handle) = self.table.get_mut(&self_) {
            UNIVERSE.with_player_mut(player_handle.0, |player| {
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

    fn drop(&mut self, rep: Resource<PlayerHandle>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}
