use jni::elements::ReleaseMode;
use jni::objects::{JByteArray, JClass, JLongArray, JObject, JString};
use jni::strings::JNIString;
use jni::sys::jlong;
use jni::{jni_str, EnvUnowned};
use wasmer::{imports, Instance, Module, Store};

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
pub extern "system" fn Java_de_cjdev_wasm_WasmStore_free<'caller>(
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
pub extern "system" fn Java_de_cjdev_wasm_WasmStore_load<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    store: jlong,
    wasm_bytes: JByteArray<'caller>
) -> jlong {
    let outcome = unowned_env.with_env(|env| -> Result<jlong, jni::errors::Error> {
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

        return Ok(Box::into_raw(Box::new(instance)) as jlong);
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_WasmInstance_run<'caller>(
    mut _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    store: jlong,
    instance: jlong,
    function_name: JString<'caller>
) -> JLongArray<'caller> {
    let store = unsafe { &mut *(store as *mut Store) };
    let instance = unsafe { &mut *(instance as *mut Instance) };
    let function_name = function_name.to_string();
    let function = instance.exports.get_function(function_name.as_str());
    _unowned_env.with_env(|env| -> Result<JLongArray, jni::errors::Error> {
        let function = match function {
            Ok(f) => f,
            Err(_) => {
                env.throw_new(
                    jni_str!("java/lang/RuntimeException"),
                    JNIString::from(format!(
                        "Function '{}' not found",
                        function_name
                    ))
                )?;
                return Ok(JLongArray::null());
            },
        };

        let results = match function.call(store, &[]) {
            Ok(r) => r,
            Err(e) => {
                env.throw_new(
                    jni_str!("java/lang/RuntimeException"),
                    JNIString::from(format!(
                        "Wasm call '{}'",
                        e
                    ))
                )?;
                return Ok(JLongArray::null());
            }
        };


        let values: Vec<i64> = results
            .iter()
            .map(|v| v.unwrap_i64())
            .collect();
        let array = env.new_long_array(values.len())?;
        array.set_region(env, 0, &values)?;
        return Ok(array);
    }).resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_WasmInstance_free<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    ptr: jlong
) {
    let ptr = ptr as *mut Instance;
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
