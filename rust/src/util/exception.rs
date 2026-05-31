use jni::{jni_str, Env};
use jni::strings::{JNIStr, JNIString};

pub struct JavaException {
    class: &'static JNIStr,
    message: String
}

impl JavaException {

    pub fn new_illegal_argument(message: String) -> JavaException {
        JavaException {
            class: jni_str!("java/lang/IllegalArgumentException"),
            message
        }
    }

    pub fn new_illegal_state(message: String) -> JavaException {
        JavaException {
            class: jni_str!("java/lang/IllegalStateException"),
            message
        }
    }

    pub fn new_opus(message: String) -> JavaException {
        JavaException {
            class: jni_str!("com/plasmoverse/opus/OpusException"),
            message
        }
    }
}

pub trait JavaExceptions<T> {

    fn or_throw(self, env: &mut Env) -> jni::errors::Result<T>;
}

impl<T> JavaExceptions<T> for Result<T, JavaException> {

    fn or_throw(self, env: &mut Env) -> jni::errors::Result<T> {
        self.map_err(|exception| {
            let _ = env.throw_new(exception.class, &JNIString::new(exception.message));
            jni::errors::Error::JavaException
        })
    }
}
