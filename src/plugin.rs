use crate::{get_logger, LoggerImpl, Plugin, PLUGINS};
use jni::objects::{JClass, JString};
use jni::EnvUnowned;
use std::fs;
use std::path::Path;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, WasmBacktraceDetails};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

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
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();

    let logger = get_logger();

    let mut linker: Linker<_> = Linker::new(&engine);
    crate::example::plugin::logging::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).unwrap();
    crate::example::plugin::bindings::add_to_linker::<PluginImpl, HasSelf<_>>(&mut linker, |state: &mut PluginImpl| state).unwrap();
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

    store.set_fuel(1_000_000u64)?;
    let plugin_id = instance.call_plugin_id(&mut store)?;

    store.data_mut().logger = Some(LoggerImpl::new(&plugin_id));

    instance.call_entry(&mut store)?;
    if let Some(logger) = &store.data().logger {
        logger.info("Success loading plugin binary");
    }

    Ok(plugin_id)
}
