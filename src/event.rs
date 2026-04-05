use crate::{api, get_logger, PLUGINS};
use jni::errors::Error;
use jni::objects::{JByteArray, JClass, ReleaseMode};
use jni::sys::{jbyte, jint};
use jni::EnvUnowned;
use std::collections::HashMap;
use std::sync::OnceLock;
use crate::plugin::PluginImpl;

pub static EVENT_NAMES: OnceLock<HashMap<String, u32>> = OnceLock::new();
pub static CUSTOM_PLAYER_COMMANDS: OnceLock<HashMap<String, i32>> = OnceLock::new();

impl crate::example::plugin::events::Host for PluginImpl {
    fn get_event_id(&mut self, name: String) -> u32 {
        EVENT_NAMES.get().expect("EVENT_NAMES not initialized").get(&name).unwrap().clone()
    }
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_eu_cj4_wasm_core_Universe_dispatch_1event<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    j_event: jint,
    j_args: JByteArray<'caller>,
    j_length: jint
) -> jint {
    let event = j_event as u32;

    let message: &[u8] = unowned_env.with_env(|env| -> Result<_, Error> {
        let auto_elems = unsafe { j_args.get_elements_critical(env, ReleaseMode::NoCopyBack)? };
        let ptr_u8 = auto_elems.as_ptr() as *const u8;
        let slice_u8 = unsafe {
            debug_assert_eq!(size_of::<jbyte>(), size_of::<u8>());
            std::slice::from_raw_parts(ptr_u8, j_length as usize)
        };
        Ok(slice_u8)
    }).resolve::<jni::errors::ThrowRuntimeExAndDefault>();

    let logger = get_logger();

    if let Ok(mut plugins) = PLUGINS.lock() {
        for plugin in plugins.iter_mut() {
            match plugin.instance.call_handle_event(&mut plugin.store, event, message) {
                Ok(result) => {
                    if result != 0 {
                        return result;
                    }
                },
                Err(err) => {
                    logger.error(format!("Plugin '{}' failed to handle event: {:#?}", plugin.plugin_id, err));
                }
            }
        }
    };
    api::event::RESULT_DEFAULT
}
