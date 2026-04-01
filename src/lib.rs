mod event;
mod plugin;
mod bindings;
mod universe;
pub mod registry;
mod api;

use crate::bindings::logger::LoggerImpl;
use crate::plugin::PluginInstance;
use jni::sys::jint;
use jni::JavaVM;
use once_cell::sync::Lazy;
use std::panic;
use std::sync::{Mutex, OnceLock};
use wasmtime::component::bindgen;

bindgen!({
    path: "wit/api.wit",
    world: "plugin",
    with: {
        //"example:plugin/bindings.player": bindings::player::PlayerHandle,
        //"example:plugin/bindings.level": bindings::level::LevelHandle
    }
});

static JVM: OnceLock<JavaVM> = OnceLock::new();
static LOGGER: OnceLock<LoggerImpl> = OnceLock::new();
static PLUGINS: Lazy<Mutex<Vec<PluginInstance>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

fn get_vm() -> &'static JavaVM {
    JVM.get().expect("JVM not initialized")
}

fn get_logger() -> &'static LoggerImpl {
    LOGGER.get().expect("LOGGER not initialized")
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut std::ffi::c_void) -> jint {
    panic::set_hook(Box::new(panic_handler));
    JVM.set(vm).expect("JVM already initialized");
    let logger = LoggerImpl::new("Wasm Rust".to_string());
    LOGGER.set(logger).expect("LOGGER already initialized");
    jni::sys::JNI_VERSION_1_8
}

pub fn panic_handler(panic_info: &panic::PanicHookInfo) {
    let logger = LOGGER.get().unwrap();
    logger.error(panic_info.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
