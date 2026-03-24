mod event;
mod plugin;
mod bindings;

use crate::bindings::logger::LoggerImpl;
use crate::plugin::PluginInstance;
use jni::errors::Error;
use jni::sys::jint;
use jni::JavaVM;
use once_cell::sync::Lazy;
use std::sync::{Mutex, OnceLock};
use wasmtime::component::bindgen;

bindgen!({
    path: "wit/api.wit",
    world: "plugin",
    with: {
        "example:plugin/player.player": bindings::player::PlayerImpl
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
    JVM.set(vm).expect("JVM already initialized");
    get_vm().attach_current_thread(|_env| {
        let logger = LoggerImpl::new("Wasm Rust".to_string());
        LOGGER.set(logger).expect("LOGGER already initialized");
        Ok::<bool, Error>(true)
    }).ok();
    jni::sys::JNI_VERSION_1_8
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
