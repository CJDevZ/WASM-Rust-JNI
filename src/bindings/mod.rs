use crate::example;
use crate::plugin::PluginImpl;
use crate::universe::Universe;
use once_cell::sync::Lazy;

pub mod player;
pub mod logger;
pub mod level;

impl example::plugin::bindings::Host for PluginImpl {}

pub static UNIVERSE: Lazy<Universe> = Lazy::new(Universe::new);

pub mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.rs"));
    pub mod core {
        include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.core.rs"));
    }
    pub mod player {
        include!(concat!(env!("OUT_DIR"), "/de.cjdev.wasm.bindings.protobuf.player.rs"));
    }
}
