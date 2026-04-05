use std::collections::HashMap;
use crate::{get_logger, LoggerImpl, Plugin, PLUGINS};
use anyhow::Error;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JByteArray, JClass, JObjectArray, JString};
use jni::EnvUnowned;
use libloading::{Library, Symbol};
use prost::Message;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, WasmBacktraceDetails};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use zip::ZipArchive;

pub struct PluginInstance {
    pub plugin_id: String,
    pub instance: Plugin,
    pub store: Store<PluginImpl>
}

pub trait LinkerExtension {
    fn extend(&self, linker: &mut Linker<PluginImpl>) -> anyhow::Result<()>;
}

pub struct PluginImpl {
    pub logger: Option<LoggerImpl>,
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
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

enum PluginType {
    File(PathBuf),
    Folder(PathBuf),
    Zip(PathBuf),
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_eu_cj4_wasm_core_WasmCore_load<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    plugin_folder: JString<'caller>,
    event_types: JObjectArray<'caller, JString<'caller>>,
    custom_player_commands: JObjectArray<'caller, JString<'caller>>,
    linker_extensions: JObjectArray<'caller, JString<'caller>>,
) -> JByteArray<'caller> {

    // EVENTS
    let (events, custom_player_commands) = unowned_env.with_env(|env| {
        let mut events: HashMap<String, u32> = HashMap::new();
        let len = event_types.len(env)?;
        for i in 0..len {
            let event_type = event_types.get_element(env, i)?.to_string();
            events.insert(event_type, (i + 1) as u32);
        }

        // CUSTOM PLAYER COMMANDS
        let mut map: HashMap<String, i32> = HashMap::new();
        let len = custom_player_commands.len(env)?;
        for i in 0..len {
            let event_type = custom_player_commands.get_element(env, i)?.to_string();
            map.insert(event_type, i as i32);
        }
        Ok::<_, jni::errors::Error>((events, map))
    }).resolve::<ThrowRuntimeExAndDefault>();
    get_logger().info(format!("{:?}", custom_player_commands));

    crate::event::EVENT_NAMES.set(events).expect("EVENT_NAMES already initialized");
    crate::event::CUSTOM_PLAYER_COMMANDS.set(custom_player_commands).expect("CUSTOM_PLAYER_COMMANDS already initialized");


    // PLUGINS
    let plugin_folder = plugin_folder.to_string();
    let folder = Path::new(&plugin_folder);
    if !folder.exists() {
        get_logger().info("Creating plugin folder");
        fs::create_dir_all(folder).expect("Failed to create plugin folder");
        return unowned_env.with_env(|env| {
            env.byte_array_from_slice(&[])
        }).resolve::<ThrowRuntimeExAndDefault>();
    }
    let entries = fs::read_dir(folder).expect("Failed to read plugin folder");
    let filtered: Vec<_> = entries.filter_map(|entry| {
        let path = entry.ok()?.path();

        if path.clone().is_dir() {
            return Some(PluginType::Folder(path));
        }
        path.clone().extension().map(|ext| {
            match ext.to_str()? {
                "wasm" => Some(PluginType::File(path)),
                "zip" => Some(PluginType::Zip(path)),
                _ => None
            }
        })?
    }).collect();
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
    config.wasm_threads(true);
    config.consume_fuel(true);
    let engine = Engine::new(&config).expect("Couldn't create engine");

    let logger = get_logger();

    let mut linker: Linker<_> = Linker::new(&engine);
    // TODO: Make sure to add all the imports to the linker <3
    crate::example::plugin::logging::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).expect("Failed to add 'example::plugin::logging' to Linker");
    crate::example::plugin::bindings::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).expect("Failed to add 'example::plugin::bindings' to Linker");
    crate::example::plugin::block_registry::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).expect("Failed to add 'example::plugin::block_registry' to Linker");
    crate::example::plugin::level::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).expect("Failed to add 'example::plugin::level' to Linker");
    crate::example::plugin::player::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).expect("Failed to add 'example::plugin::player' to Linker");
    crate::example::plugin::commands::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).expect("Failed to add 'example::plugin::commands' to Linker");
    crate::example::plugin::events::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).expect("Failed to add 'example::plugin::events' to Linker");
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("Failed to add wasi to Linker");
    //unowned_env.with_env(|env| {
    //    let len = linker_extensions.len(env)?;
    //    for i in 0..len {
    //        let extension_lib = linker_extensions.get_element(env, i)?.to_string();
    //        unsafe {
    //            let lib = Library::new(&extension_lib)?;
    //            let func: Symbol<unsafe extern "C" fn(&mut Linker<PluginImpl>)> =
    //                lib.get(b"register_extension")?;
    //            func(&mut linker);
    //        }
    //    }
    //    Ok::<(), Error>(())
    //}).into_outcome();

    let mut plugins: Vec<PluginInstance> = Vec::new();
    let mut pluginData: Vec<crate::bindings::protobuf::core::plugin::Plugin> = Vec::new();

    for path in filtered {
        let (plugin_id, instance, store, proto_data) = match load_plugin(&path, &engine, &linker) {
            Ok(i) => i,
            Err(err) => {
                logger.error(format!("{:#?}", err));
                continue;
            }
        };

        logger.info(&proto_data.id);
        pluginData.push(proto_data);

        plugins.push(PluginInstance {
            plugin_id,
            instance,
            store,
        });
    }

    match PLUGINS.lock() {
        Ok(mut vec) => {
            vec.append(&mut plugins);
            logger.info(format!("Loaded {} plugins", vec.len()))
        }
        Err(_) => {}
    }

    unowned_env.with_env(|env| {
        let mut message = Vec::new();
        crate::bindings::protobuf::core::plugin::PluginRegistration {
            plugins: pluginData
        }.encode(&mut message).expect("Failed to encode PluginRegistration");
        env.byte_array_from_slice(&message)
    }).resolve::<ThrowRuntimeExAndDefault>()
}

fn load_plugin(plugin: &PluginType, engine: &Engine, linker: &Linker<PluginImpl>) -> Result<(String, Plugin, Store<PluginImpl>, crate::bindings::protobuf::core::plugin::Plugin), Error> {
    let (wasm_bytes, data_path) = match plugin {
        PluginType::File(path) => {
            (fs::read(&path)?, None)
        }
        PluginType::Folder(path) => {
            let wasm = path.join("component.wasm");
            (fs::read(&wasm)?, Some(path.to_str().ok_or_else(|| Error::msg("Failed getting OsStr"))?.to_string()))
        }
        PluginType::Zip(path) => {
            let file = File::open(path)?;
            let mut archive = ZipArchive::new(file)?;

            let mut wasm = archive.by_name("component.wasm")?;
            let mut bytes = Vec::new();
            wasm.read_to_end(&mut bytes)?;

            (bytes, Some(path.to_str().ok_or_else(|| Error::msg("Failed getting OsStr"))?.to_string()))
        }
    };
    let component = Component::new(&engine, &wasm_bytes)?;

    let wasi_ctx = WasiCtx::builder().inherit_env().build();
    let plugin = PluginImpl::new(wasi_ctx);
    let mut store = Store::new(&engine, plugin);

    let instance = Plugin::instantiate(&mut store, &component, &linker)?;
    let plugin_id = load_wasm(&mut store, &instance)?;

    let proto_data = crate::bindings::protobuf::core::plugin::Plugin {
        id: plugin_id.clone(),
        path: data_path
    };

    Ok((plugin_id, instance, store, proto_data))
}

pub fn load_wasm<'caller>(
    mut store: &mut Store<PluginImpl>,
    instance: &Plugin
) -> Result<String, Error> {

    store.set_fuel(unsafe{crate::bindings::FUEL_CAP})?;
    let plugin_id = instance.call_plugin_id(&mut store)?;

    store.data_mut().logger = Some(LoggerImpl::new(&plugin_id));

    instance.call_entry(&mut store)?;
    if let Some(logger) = &store.data().logger {
        logger.info("Success loading plugin binary");
    }

    Ok(plugin_id)
}
