use crate::bindings::player::ShadowPlayer;
use crate::bindings::protobuf::core::UniverseChanges;
use crate::bindings::{Syncable, UNIVERSE};
use jni::errors::{Error, ThrowRuntimeExAndDefault};
use jni::objects::{JByteArray, JClass};
use jni::sys::jlong;
use jni::EnvUnowned;
use prost::Message;
use slotmap::{DefaultKey, Key, KeyData, SlotMap};
use std::sync::RwLock;
use crate::bindings::level::ShadowLevel;
use crate::example::plugin::level::LevelHandle;
use crate::PLUGINS;

pub struct Universe {
    pub players: UniverseStorage<ShadowPlayer>,
    pub levels: UniverseStorage<ShadowLevel>
}

pub struct UniverseStorage<T: Syncable> {
    map: RwLock<SlotMap<DefaultKey, T>>,
}

impl <T: Syncable> UniverseStorage<T> {
    fn new() -> UniverseStorage<T> {
        UniverseStorage {
            map: RwLock::new(SlotMap::new()),
        }
    }

    pub fn with<R>(&self, uid: u64, f: impl FnOnce(&T) -> R) -> Option<R> {
        let map = self.map.read().ok()?;
        map.get(KeyData::from_ffi(uid).into()).map(f)
    }

    pub fn with_mut<R>(&self, uid: u64, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let mut map = self.map.write().ok()?;
        map.get_mut(KeyData::from_ffi(uid).into()).map(f)
    }

    pub fn add(&self, value: T) -> u64 {
        let mut map = self.map.write().ok().unwrap();
        map.insert(value).data().as_ffi()
    }

    pub fn remove(&self, uid: u64) {
        if let Some(mut map) = self.map.write().ok() {
            map.remove(KeyData::from_ffi(uid).into());
        }
    }

    fn flush_all(&self) -> Vec<T::Change> {
        let mut map = self.map.write().unwrap();
        map.iter_mut()
            .filter_map(|(uid, entity)| entity.encode_changes(uid.data().as_ffi()))
            .collect()
    }
}

impl Universe {
    pub fn new() -> Self {
        Universe {
            players: UniverseStorage::new(),
            levels: UniverseStorage::new()
        }
    }
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_Universe_add_1player<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    level_handle: jlong
) -> jlong {
    UNIVERSE.players.add(ShadowPlayer::new(level_handle as LevelHandle)) as jlong
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_Universe_remove_1player<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    universal_id: jlong
) {
    UNIVERSE.players.remove(universal_id as u64)
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_Universe_add_1level<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>
) -> jlong {
    UNIVERSE.levels.add(ShadowLevel::new()) as jlong
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_Universe_push_1changes<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    message: JByteArray<'caller>
) {
    let mut plugins = PLUGINS.lock().unwrap();
    for plugin in plugins.iter_mut() {
        plugin.store.set_fuel(unsafe{crate::bindings::FUEL_CAP}).expect("Failed to set fuel capacity");
    }
    unowned_env.with_env(|env| {
        let buf: Vec<u8> = env.convert_byte_array(message)?;
        let changes: UniverseChanges = UniverseChanges::decode(&*buf).expect("Failed to decode UniverseChanges");

        for player_change in changes.player_changes {
            UNIVERSE.players.with_mut(player_change.universal_id, |player| {
                player.decode_changes(player_change);
            });
        }

        Ok::<(), Error>(())
    }).resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_Universe_fetch_1changes<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>
) -> JByteArray<'caller> {
    let mut buf: Vec<u8> = Vec::new();

    let changes = UniverseChanges {
        player_changes: UNIVERSE.players.flush_all(),
        level_changes: UNIVERSE.levels.flush_all()
    };
    changes.encode(&mut buf).expect("Failed encoding UniverseChanges");

    unowned_env.with_env(|env| {
        env.byte_array_from_slice(&buf)
    }).resolve::<ThrowRuntimeExAndDefault>()
}
