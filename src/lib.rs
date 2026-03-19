use jni::elements::ReleaseMode;
use jni::objects::{JByteArray, JClass, JObject};
use jni::sys::{jboolean, jlong, JNI_TRUE};
use jni::EnvUnowned;
use wasmer::{imports, Instance, Module, Store, Value};

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_WasmStore_default_1store<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>
) -> *mut Store {
    Box::into_raw(Box::new(Store::default()))
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_WasmStore_free_1store<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    ptr: jlong
) {
    let ptr = ptr as *mut Store;
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_WasmNative_run_1wasm<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    store: jlong,
    wasm_bytes: JByteArray<'caller>
) -> jboolean {

    let outcome = unowned_env.with_env(|env| -> Result<jboolean, jni::errors::Error> {
        let store = unsafe { &mut *(store as *mut Store) };
        let elements = unsafe {
            &wasm_bytes.get_elements(env, ReleaseMode::NoCopyBack)?
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(elements.as_ptr() as *const u8, elements.len())
        };

        let module = Module::new(&store, bytes).unwrap();
        // The module doesn't import anything, so we create an empty import object.
        let import_object = imports! {};
        let instance = Instance::new(store, &module, &import_object).unwrap();

        let sum_fn = instance.exports.get_function("main").unwrap();
        sum_fn.call(store, &[]).unwrap();
        return Ok(JNI_TRUE);
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
