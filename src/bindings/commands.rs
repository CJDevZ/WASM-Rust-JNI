use crate::example::plugin::commands::FunctionHandle;
use crate::plugin::PluginImpl;

impl crate::example::plugin::commands::Host for PluginImpl {
    fn register_command(&mut self, command: String, function_handle: FunctionHandle) -> () {
        todo!()
    }
}
