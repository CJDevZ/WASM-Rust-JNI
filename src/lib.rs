use crate::example::plugin;
use crate::example::plugin::logging::Host;
use crate::exports::example::plugin::events::{Event, MessageEvent};
use jni::errors::Error;
use jni::objects::{Global, JClass, JObject, JObjectArray, JString};
use jni::sys::jint;
use jni::{jni_sig, jni_str, EnvUnowned, JValue, JavaVM};
use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use wasmtime::component::{bindgen, Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Config, Engine, Store, WasmBacktraceDetails};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

bindgen!({
    path: "wit/api.wit",
    world: "plugin",
    with: {
        "example:plugin/player.player": PlayerImpl
    }
});

struct PluginInstance {
    plugin_id: String,
    instance: Plugin,
    store: Store<PluginImpl>
}

unsafe impl Send for PluginInstance {}

pub struct PluginImpl {
    logger: Option<LoggerImpl>,
    wasi_ctx: WasiCtx,
    table: ResourceTable,
}

impl PluginImpl {
    pub fn new(wasi_ctx: WasiCtx) -> Self {
        Self {
            logger: None,
            wasi_ctx,
            table: ResourceTable::new(),
        }
    }
}

impl WasiView for PluginImpl {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table
        }
    }
}

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
pub struct PlayerImpl(Global<JObject<'static>>);

impl plugin::player::HostPlayer for PluginImpl {
    fn send_message(&mut self, self_: Resource<PlayerImpl>, msg: String) -> () {
        let mut player: &JObject = &self.table.get(&self_).unwrap().0;

        let vm = get_vm();
        vm.attach_current_thread(|env| {
            let java_msg = env.new_string(msg)?;

            env.call_method(
                &mut player,
                jni_str!("wasm$send_message"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::from(&java_msg)],
            )?;

            Ok::<(), Error>(())
        }).ok();
    }

    fn drop(&mut self, rep: Resource<plugin::player::Player>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl plugin::player::Host for PluginImpl {}

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



#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_Wasm_load<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    plugin_folder: JString<'caller>
) {
    let plugin_folder = plugin_folder.to_string();
    let folder = Path::new(&plugin_folder);
    if !folder.exists() {
        let logger = get_logger();
        logger.info("Creating plugin folder");
        get_logger().info("Creating plugin folder");
        fs::create_dir_all(folder).ok();
        return;
    }
    let entries = fs::read_dir(folder).unwrap();
    let filtered: Vec<_> = entries.filter_map(|entry| {
        let path = entry.ok()?.path();

        if path.extension().and_then(|ext| ext.to_str()) == Some("wasm") {
            Some(path)
        } else {
            None
        }
    }).collect();
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
    config.wasm_threads(true);
    let engine = Engine::new(&config).unwrap();

    let logger = get_logger();

    let mut linker: Linker<_> = Linker::new(&engine);
    plugin::logging::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).unwrap();
    plugin::player::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).unwrap();
    if let Err(err) = add_to_linker_sync(&mut linker) {
        logger.error(format!("Failed to add wasi to linker: {:#?}", err));
        return;
    }

    let mut plugins: Vec<PluginInstance> = Vec::new();

    for path in filtered {
        let wasm_bytes = fs::read(&path).unwrap();
        let component = match Component::new(&engine, &wasm_bytes) {
            Ok(c) => c,
            Err(err) => {
                logger.error(format!("{:#?}", err));
                continue
            }
        };

        let wasi_ctx = WasiCtx::builder().inherit_env().build();
        let plugin = PluginImpl::new(wasi_ctx);
        let mut store = Store::new(&engine, plugin);

        let instance = match Plugin::instantiate(&mut store, &component, &linker) {
            Ok(i) => i,
            Err(err) => {
                logger.error(format!("{:#?}", err));
                continue
            }
        };

        let path_str = path.to_str().unwrap();
        let plugin_id = load_wasm(&mut store, &instance);
        let plugin_id = match plugin_id {
            Ok(s) => s,
            Err(err) => {
                logger.error(format!("Failed loading plugin binary for '{}': {:#?}", path_str, err));
                continue
            }
        };
        logger.info(path_str);
        plugins.push(PluginInstance {
            plugin_id,
            instance,
            store
        });
    }

    match PLUGINS.lock() {
        Ok(mut vec) => {
            vec.append(&mut plugins);
            logger.info(format!("Loaded {} plugins", vec.len()))
        }
        Err(_) => {}
    }
}

pub fn load_wasm<'caller>(
    mut store: &mut Store<PluginImpl>,
    instance: &Plugin
) -> Result<String, anyhow::Error> {

    let plugin_id = instance.call_plugin_id(&mut store)?;

    store.data_mut().logger = Some(LoggerImpl::new(&plugin_id));

    instance.call_entry(&mut store)?;
    if let Some(logger) = &store.data().logger {
        logger.info("Success loading plugin binary");
    }

    Ok(plugin_id)
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_Wasm_dispatch_1event<'caller>(
    mut _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    j_event: JString<'caller>,
    j_args: JObjectArray<'caller>,
) {
    let event_name = j_event.to_string();
    _unowned_env.with_env(|env| -> Result<(), Error> {

        let logger = get_logger();
        if let Ok(mut plugins) = PLUGINS.lock() {
            let event_name_str: &str = event_name.as_str();
            for plugin in plugins.iter_mut() {
                let state = plugin.store.data_mut();

                let event: Event = match event_name_str {
                    "Tick" => {
                        Event::Tick
                    },
                    "TickPlayer" => {
                        let obj_ref = j_args.get_element(env, 0).unwrap();
                        let global_ref = env.new_global_ref(obj_ref).unwrap();
                        let player = PlayerImpl(global_ref);
                        Event::TickPlayer(state.table.push(player).unwrap())
                    },
                    "Message" => unsafe {
                        let obj_ref = j_args.get_element(env, 0).unwrap();
                        let message = JString::from_raw(env, *obj_ref);
                        Event::Message(MessageEvent {
                            message: Some(message.to_string()),
                        })
                    },
                    &_ => {
                        return Err(Error::ParseFailed(event_name));
                    }
                };

                if let Err(err) = plugin.instance.example_plugin_events().call_handle_event(&mut plugin.store, &event) {
                    logger.error(format!("Plugin '{}' failed to handle event: {:#?}", plugin.plugin_id, err));
                }
            }
        }

        Ok(())
    }).resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
