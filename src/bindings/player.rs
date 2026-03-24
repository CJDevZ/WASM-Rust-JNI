use jni::{jni_sig, jni_str, JValue};
use jni::errors::Error;
use jni::objects::{Global, JObject};
use wasmtime::component::Resource;
use crate::{example, get_vm};
use crate::plugin::PluginImpl;

pub struct PlayerImpl(pub Global<JObject<'static>>);

impl example::plugin::player::HostPlayer for PluginImpl {
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

    fn drop(&mut self, rep: Resource<example::plugin::player::Player>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl example::plugin::player::Host for PluginImpl {}
