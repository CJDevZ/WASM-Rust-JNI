mod event;
mod plugin;
mod bindings;

use crate::example::plugin::logging::Host;
use crate::plugin::{PluginImpl, PluginInstance};
use jni::errors::Error;
use jni::objects::{Global, JObject};
use jni::sys::jint;
use jni::{jni_sig, jni_str, JValue, JavaVM};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex, OnceLock};
use wasmtime::component::bindgen;

bindgen!({
    path: "wit/api.wit",
    world: "plugin",
    with: {
        "example:plugin/player.player": bindings::player::PlayerImpl
    }
});

impl Host for PluginImpl {
    fn info(&mut self, msg: String) -> () {
        if let Some(logger) = &self.logger {
            logger.info(msg);
        }
    }

    fn warn(&mut self, msg: String) -> () {
        if let Some(logger) = &self.logger {
            logger.warn(msg);
        }
    }

    fn error(&mut self, msg: String) -> () {
        if let Some(logger) = &self.logger {
            logger.error(msg);
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoggerImpl(Arc<Global<JObject<'static>>>);

impl LoggerImpl {
    fn new(name: impl AsRef<str>) -> Self {
        let vm = get_vm();
        vm.attach_current_thread(|env| {
            let name = env.new_string(name)?;
            let class = env.find_class(jni_str!("org/slf4j/LoggerFactory"))?;
            let result = env.call_static_method(class, jni_str!("getLogger"), jni_sig!("(Ljava/lang/String;)Lorg/slf4j/Logger;"), &[JValue::from(&name)])?;
            let obj = result.l()?;
            let global = env.new_global_ref(obj)?;
            Ok::<LoggerImpl, Error>(LoggerImpl(Arc::new(global)))
        }).expect("Failed to create logger")
    }

    fn info(&self, msg: impl AsRef<str>) {
        let vm = get_vm();
        vm.attach_current_thread(|env| {
            let java_msg = env.new_string(msg)?;

            env.call_method(
                self.0.as_obj(),
                jni_str!("info"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::from(&java_msg)],
            )?;

            Ok::<(), Error>(())
        }).ok();
    }

    fn warn(&self, msg: impl AsRef<str>) {
        let vm = get_vm();
        vm.attach_current_thread(|env| {
            let java_msg = env.new_string(msg)?;

            env.call_method(
                self.0.as_obj(),
                jni_str!("warn"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::from(&java_msg)],
            )?;

            Ok::<(), Error>(())
        }).ok();
    }

    fn error(&self, msg: impl AsRef<str>) {
        let vm = get_vm();
        vm.attach_current_thread(|env| {
            let java_msg = env.new_string(msg)?;

            env.call_method(
                self.0.as_obj(),
                jni_str!("error"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::from(&java_msg)],
            )?;

            Ok::<(), Error>(())
        }).ok();
    }
}

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
