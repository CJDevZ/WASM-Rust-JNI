use crate::example;
use crate::plugin::PluginImpl;
use crate::universe::Universe;
use jni::objects::JClass;
use jni::sys::jlong;
use jni::EnvUnowned;
use once_cell::sync::Lazy;

pub mod player;
pub mod logger;
pub mod level;

impl example::plugin::bindings::Host for PluginImpl {}

pub static UNIVERSE: Lazy<Universe> = Lazy::new(Universe::new);
pub static mut FUEL_CAP: u64 = 0u64;

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_WasmCore_set_1fuel_1cap<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    fuel_cap: jlong
) {
    unsafe {
        FUEL_CAP = fuel_cap as u64;
    }
}

pub trait Syncable {
    type Change;

    fn encode_changes(&mut self, universal_id: u64) -> Option<Self::Change>;
    fn decode_changes(&mut self, change: Self::Change);
}

pub mod protobuf {
    pub mod core {
        include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.core.rs"));

        pub mod player {
            include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.core.player.rs"));
        }
        pub mod plugin {
            include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.core.plugin.rs"));
        }
        pub mod sync {
            include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.core.sync.rs"));
        }
    }
}
