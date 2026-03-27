use crate::bindings::protobuf::PlayerChange;
use crate::bindings::{protobuf, UNIVERSE};
use crate::example;
use crate::example::plugin::bindings::{BlockPos, TextComponent, Vec3};
use crate::plugin::PluginImpl;
use slotmap::{DefaultKey, Key};
use wasmtime::component::Resource;
use crate::bindings::level::LevelHandle;

#[derive(Clone, Copy)]
pub struct PlayerHandle(pub u64);

pub struct ShadowPlayer {
    pub uid: DefaultKey,
    level_handle: LevelHandle,
    position: Vec3,
    pub message_queue: Vec<TextComponent>,
    pub dirty: u64
}

impl ShadowPlayer {
    const POSITION_DIRTY: u64 = 1 << 0;
    const MESSAGE_QUEUE_DIRTY: u64 = 1 << 1;

    pub fn new(uid: DefaultKey, level_handle: LevelHandle) -> Self {
        Self {
            uid,
            level_handle,
            position: Vec3 {x: 0f64, y: 0f64, z: 0f64},
            message_queue: Vec::new(),
            dirty: 0
        }
    }

    pub fn decode_changes(&mut self, change: PlayerChange) {
        if let Some(position) = change.position {
            self.position = Vec3 {x: position.x, y: position.y, z: position.z };
        }
    }

    pub fn encode_changes(&self) -> Option<PlayerChange> {
        if self.dirty == 0 {
            return None
        }

        let position = if self.dirty & ShadowPlayer::POSITION_DIRTY != 0 {
            Some(protobuf::Vec3 {
                x: self.position.x,
                y: self.position.y,
                z: self.position.z
            })
        } else { None };

        let message_queue = self.message_queue.iter().map(|m| {
            match m {
                TextComponent::Plain(msg) => {
                    protobuf::TextComponent {
                        r#type: protobuf::text_component::Type::Plain as i32,
                        message: msg.clone()
                    }
                }
                TextComponent::MiniMessage(msg) => {
                    protobuf::TextComponent {
                        r#type: protobuf::text_component::Type::MiniMessage as i32,
                        message: msg.clone()
                    }
                }
                TextComponent::Json(msg) => {
                    protobuf::TextComponent {
                        r#type: protobuf::text_component::Type::Json as i32,
                        message: msg.clone()
                    }
                }
            }
        }).collect();

        Some(PlayerChange {
            universal_id: self.uid.data().as_ffi(),
            position,
            message_queue
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

    fn send_message(&mut self, self_: Resource<PlayerHandle>, msg: TextComponent) -> () {
        let player_handle: &PlayerHandle = self.table.get(&self_).unwrap();
        UNIVERSE.with_player_mut(player_handle.0, |player| {
            player.message_queue.push(msg);
            player.dirty |= ShadowPlayer::MESSAGE_QUEUE_DIRTY;
        });
        // let vm = get_vm();
        // vm.attach_current_thread(|env| {
        //     let java_msg = env.new_string(msg)?;
        //
        //     env.call_method(
        //         &mut player,
        //         jni_str!("wasm$send_message"),
        //         jni_sig!("(Ljava/lang/String;)V"),
        //         &[JValue::from(&java_msg)],
        //     )?;
        //
        //     Ok::<(), Error>(())
        // }).ok();
    }

    fn teleport(&mut self, self_: Resource<PlayerHandle>, x: f64, y: f64, z: f64) -> () {
        if let Ok(player_handle) = self.table.get_mut(&self_) {
            UNIVERSE.with_player_mut(player_handle.0, |player| {
                player.position = Vec3 { x, y, z };
                player.dirty |= ShadowPlayer::POSITION_DIRTY;
            });
        }
    }

    fn drop(&mut self, rep: Resource<PlayerHandle>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}
