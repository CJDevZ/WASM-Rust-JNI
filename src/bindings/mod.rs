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
pub extern "system" fn Java_de_cjdev_wasm_Wasm_set_1fuel_1cap<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    fuel_cap: jlong
) {
    unsafe {
        FUEL_CAP = fuel_cap as u64;
    }
}

pub mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.rs"));
    pub mod core {
        include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.core.rs"));
    }
    pub mod player {
        include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.player.rs"));
    }
    pub mod plugin {
        include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.plugin.rs"));
    }
}
