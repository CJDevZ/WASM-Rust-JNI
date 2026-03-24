use jni::EnvUnowned;
use jni::errors::Error;
use jni::objects::{JClass, JObjectArray, JString};
use crate::{get_logger, PLUGINS};
use crate::bindings::player::PlayerImpl;
use crate::exports::example::plugin::events::{Event, MessageEvent};

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
