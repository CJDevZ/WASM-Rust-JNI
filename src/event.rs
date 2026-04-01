use crate::{get_logger, PLUGINS};
use jni::errors::Error;
use jni::objects::{JClass, JLongArray, ReleaseMode};
use jni::sys::{jint, jlong};
use jni::EnvUnowned;

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_WasmCore_dispatch_1event<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    j_event: jint,
    j_args: JLongArray<'caller>,
) {
    let event = j_event as u32;

    let args_u64: &[u64] = unowned_env.with_env(|env| -> Result<_, Error> {
        let len = j_args.len(env)?;
        let auto_elems = unsafe { j_args.get_elements_critical(env, ReleaseMode::NoCopyBack)? };
        let ptr_u64 = auto_elems.as_ptr() as *const u64;
        let slice_u64 = unsafe {
            debug_assert_eq!(size_of::<jlong>(), size_of::<u64>());
            std::slice::from_raw_parts(ptr_u64, len)
        };
        Ok(slice_u64)
    }).resolve::<jni::errors::ThrowRuntimeExAndDefault>();

    let logger = get_logger();
    if let Ok(mut plugins) = PLUGINS.lock() {
        for plugin in plugins.iter_mut() {
            if let Err(err) = plugin.instance.example_plugin_events().call_handle_event(&mut plugin.store, event, args_u64) {
                logger.error(format!("Plugin '{}' failed to handle event: {:#?}", plugin.plugin_id, err));
            }
        }
    };
}
