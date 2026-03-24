use jni::{jni_sig, jni_str, JValue};
use jni::errors::Error;
use jni::objects::{Global, JObject};
use crate::example::plugin::logging::Host;
use crate::get_vm;
use crate::plugin::PluginImpl;

#[derive(Debug)]
pub struct LoggerImpl(Global<JObject<'static>>);

impl LoggerImpl {
    pub fn new(name: impl AsRef<str>) -> Self {
        let vm = get_vm();
        vm.attach_current_thread(|env| {
            let name = env.new_string(name)?;
            let class = env.find_class(jni_str!("org/slf4j/LoggerFactory"))?;
            let result = env.call_static_method(class, jni_str!("getLogger"), jni_sig!("(Ljava/lang/String;)Lorg/slf4j/Logger;"), &[JValue::from(&name)])?;
            let obj = result.l()?;
            let global = env.new_global_ref(obj)?;
            Ok::<LoggerImpl, Error>(LoggerImpl(global))
        }).expect("Failed to create logger")
    }

    pub fn info(&self, msg: impl AsRef<str>) {
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

    pub fn warn(&self, msg: impl AsRef<str>) {
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

    pub fn error(&self, msg: impl AsRef<str>) {
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
