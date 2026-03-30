use crate::{get_logger, LoggerImpl, Plugin, PLUGINS};
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JByteArray, JClass, JString};
use jni::EnvUnowned;
use prost::Message;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use anyhow::Error;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, WasmBacktraceDetails};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use zip::ZipArchive;

pub struct PluginInstance {
    pub plugin_id: String,
    pub instance: Plugin,
    pub store: Store<PluginImpl>
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
    Raw(PathBuf),
    Zipped(PathBuf),
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_Wasm_load<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    plugin_folder: JString<'caller>
) -> JByteArray<'caller> {
    let plugin_folder = plugin_folder.to_string();
    let folder = Path::new(&plugin_folder);
    if !folder.exists() {
        let logger = get_logger();
        logger.info("Creating plugin folder");
        get_logger().info("Creating plugin folder");
        fs::create_dir_all(folder).ok();
        return unowned_env.with_env(|env| {
            env.byte_array_from_slice(&[])
        }).resolve::<ThrowRuntimeExAndDefault>();
    }
    let entries = fs::read_dir(folder).unwrap();
    let filtered: Vec<_> = entries.filter_map(|entry| {
        let path = entry.ok()?.path();

        path.clone().extension().map(|ext| {
            match ext.to_str()? {
                "wasm" => Some(PluginType::Raw(path)),
                "zip" => Some(PluginType::Zipped(path)),
                _ => None
            }
        })?
    }).collect();
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
    config.wasm_threads(true);
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();

    let logger = get_logger();

    let mut linker: Linker<_> = Linker::new(&engine);
    crate::example::plugin::logging::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).unwrap();
    crate::example::plugin::bindings::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).unwrap();
    if let Err(err) = add_to_linker_sync(&mut linker) {
        logger.error(format!("Failed to add wasi to linker: {:#?}", err));
        return unowned_env.with_env(|env| {
            env.byte_array_from_slice(&[])
        }).resolve::<ThrowRuntimeExAndDefault>();
    }

    let mut plugins: Vec<PluginInstance> = Vec::new();
    let mut pluginData: Vec<crate::bindings::protobuf::plugin::Plugin> = Vec::new();

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
        crate::bindings::protobuf::plugin::PluginRegistration {
            plugins: pluginData
        }.encode(&mut message).expect("Failed to encode PluginRegistration");
        env.byte_array_from_slice(&message)
    }).resolve::<ThrowRuntimeExAndDefault>()
}

fn load_plugin(plugin: &PluginType, engine: &Engine, linker: &Linker<PluginImpl>) -> Result<(String, Plugin, Store<PluginImpl>, crate::bindings::protobuf::plugin::Plugin), Error> {
    let (wasm_bytes, data_path) = match plugin {
        PluginType::Raw(path) => {
            (fs::read(&path)?, None)
        }
        PluginType::Zipped(path) => {
            let file = File::open(path)?;
            let mut archive = ZipArchive::new(file)?;

            let mut wasm = archive.by_name("component.wasm")?;
            let mut bytes = Vec::new();
            wasm.read_to_end(&mut bytes)?;

            (bytes, Some(path.to_str().unwrap().to_string()))
        }
    };
    let component = Component::new(&engine, &wasm_bytes)?;

    let wasi_ctx = WasiCtx::builder().inherit_env().build();
    let plugin = PluginImpl::new(wasi_ctx);
    let mut store = Store::new(&engine, plugin);

    let instance = Plugin::instantiate(&mut store, &component, &linker)?;
    let plugin_id = load_wasm(&mut store, &instance)?;

    let proto_data = crate::bindings::protobuf::plugin::Plugin {
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
