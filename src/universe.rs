use crate::bindings::player::ShadowPlayer;
use crate::bindings::protobuf::UniverseChanges;
use crate::bindings::UNIVERSE;
use jni::errors::{Error, ThrowRuntimeExAndDefault};
use jni::objects::{JByteArray, JClass};
use jni::sys::jlong;
use jni::EnvUnowned;
use prost::Message;
use slotmap::{DefaultKey, Key, KeyData, SlotMap};
use std::sync::RwLock;
use crate::bindings::level::{LevelHandle, ShadowLevel};

pub struct Universe {
    players: RwLock<SlotMap<DefaultKey, ShadowPlayer>>,
    levels: RwLock<SlotMap<DefaultKey, ShadowLevel>>
}

impl Universe {
    pub fn new() -> Self {
        Universe {
            players: RwLock::new(SlotMap::new()),
            levels: RwLock::new(SlotMap::new())
        }
    }

    pub fn with_player<R>(&self, uid: u64, f: impl FnOnce(&ShadowPlayer) -> R) -> Option<R> {
        let key = KeyData::from_ffi(uid).into();
        let players = self.players.read().ok()?;
        let player = players.get(key)?;
        Some(f(player))
    }

    pub fn with_player_mut<R>(&self, uid: u64, f: impl FnOnce(&mut ShadowPlayer) -> R) -> Option<R> {
        let key = KeyData::from_ffi(uid).into();
        let mut players = self.players.write().ok()?;
        let player = players.get_mut(key)?;
        Some(f(player))
    }

    pub fn with_players<R>(&self, f: impl Fn(&ShadowPlayer) -> R) -> Vec<R> {
        self.players.read().unwrap().iter().map(|(_, player)| {
            f(player)
        }).collect()
    }

    pub fn with_players_mut<R>(&self, f: impl Fn(&mut ShadowPlayer) -> Option<R>) -> Vec<R> {
        self.players.write().unwrap().iter_mut().filter_map(|(_, player)| {
            f(player)
        }).collect()
    }

    pub fn with_level_mut<R>(&self, uid: u64, f: impl FnOnce(&mut ShadowLevel) -> R) -> Option<R> {
        let key = KeyData::from_ffi(uid).into();
        let mut levels = self.levels.write().ok()?;
        let level = levels.get_mut(key)?;
        Some(f(level))
    }

    pub fn with_levels_mut<R>(&self, f: impl Fn(&mut ShadowLevel) -> Option<R>) -> Vec<R> {
        self.levels.write().unwrap().iter_mut().filter_map(|(_, level)| {
            f(level)
        }).collect()
    }

    pub fn add_player(&self, level_handle: LevelHandle) -> DefaultKey {
        self.players.write().unwrap().insert_with_key(|key| ShadowPlayer::new(key, level_handle))
    }

    pub fn remove_player(&self, player: DefaultKey) {
        if let Ok(mut players) = self.players.write() {
            players.remove(player);
        }
    }

    pub fn add_level(&self) -> DefaultKey {
        self.levels.write().unwrap().insert_with_key(|key| ShadowLevel::new(key))
    }

    pub fn remove_level(&self, level: DefaultKey) {
        if let Ok(mut levels) = self.levels.write() {
            levels.remove(level);
        }
    }
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_Universe_add_1player<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    level_handle: jlong
) -> jlong {
    let level_handle = LevelHandle(level_handle as u64);
    UNIVERSE.add_player(level_handle).data().as_ffi() as jlong
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_Universe_add_1level<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>
) -> jlong {
    UNIVERSE.add_level().data().as_ffi() as jlong
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_Universe_push_1changes<'caller>(
    mut _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    message: JByteArray<'caller>
) {
    _unowned_env.with_env(|env| {
        let buf: Vec<u8> = env.convert_byte_array(message)?;
        let changes: UniverseChanges = UniverseChanges::decode(&*buf).unwrap();

        for player_change in changes.player_changes {
            UNIVERSE.with_player_mut(player_change.universal_id, |player| {
                player.decode_changes(player_change);
            });
        }

        Ok::<(), Error>(())
    }).resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_Universe_fetch_1changes<'caller>(
    mut _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>
) -> JByteArray<'caller> {
    let mut buf: Vec<u8> = Vec::new();

    let player_changes = UNIVERSE.with_players_mut(|player| {
        if let Some(changes) = player.encode_changes() {
            player.dirty = 0;
            player.message_queue.clear();
            return Some(changes);
        }
        None
    });

    let level_changes = UNIVERSE.with_levels_mut(|level| {
        if let Some(changes) = level.encode_changes() {
            level.dirty = 0;
            level.block_update_queue.clear();
            return Some(changes);
        }
        None
    });

    let changes = UniverseChanges {
        player_changes,
        level_changes
    };
    changes.encode(&mut buf).expect("Failed encoding UniverseChanges");

    _unowned_env.with_env(|env| {
        env.byte_array_from_slice(&buf)
    }).resolve::<ThrowRuntimeExAndDefault>()
}
