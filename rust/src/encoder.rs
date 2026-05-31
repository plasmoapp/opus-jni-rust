use std::cmp::{max, min};
use jni::objects::{JByteArray, JClass, JObject, JShortArray, JValue};
use jni::{jni_sig, jni_str, Env, EnvUnowned};
use jni::errors::ThrowRuntimeExAndDefault;
use jni::sys::{jboolean, jint, jlong, jshort};
use opus::{Application, Bitrate, Channels, Encoder};
use crate::encoder_container::EncoderContainer;
use crate::util::exception::{JavaException, JavaExceptions};
use crate::util::into_exception::ErrIntoException;
use crate::util::pointer::{get_pointer_from_field, JavaPointers};

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusEncoder_createNative<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    sample_rate: jint,
    stereo: jboolean,
    opus_mode: jint,
    mtu_size: jint
) -> jlong {
    env.with_env(|env| create_encoder(sample_rate, stereo, opus_mode, mtu_size).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusEncoder_resetNative<'local>(
    mut env: EnvUnowned<'local>,
    encoder: JObject<'local>
) {
    env.with_env(|env| encoder_reset(env, encoder).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusEncoder_closeNative<'local>(
    mut env: EnvUnowned<'local>,
    encoder: JObject<'local>
) {
    env.with_env(|env| encoder_close(env, encoder).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusEncoder_encodeNative<'local>(
    mut env: EnvUnowned<'local>,
    encoder: JObject<'local>,
    samples: JShortArray<'local>
) -> JByteArray<'local> {
    env.with_env(|env| encoder_encode(env, encoder, samples).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusEncoder_setBitrateNative<'local>(
    mut env: EnvUnowned<'local>,
    encoder: JObject<'local>,
    bitrate: jint
) {
    env.with_env(|env| encoder_set_bitrate(env, encoder, bitrate).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusEncoder_getBitrateNative<'local>(
    mut env: EnvUnowned<'local>,
    encoder: JObject<'local>,
) -> jint {
    env.with_env(|env| encoder_get_bitrate(env, encoder).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}


fn create_encoder(
    sample_rate: jint,
    stereo: jboolean,
    opus_mode: jint,
    mtu_size: jint
) -> Result<jlong, JavaException> {
    let channels = if stereo { Channels::Stereo } else { Channels::Mono };

    let mode = match opus_mode {
        2049 => Application::Audio,
        2051 => Application::LowDelay,
        _ => Application::Voip
    };

    let encoder = Encoder::new(sample_rate as u32, channels, mode)
        .err_into_opus_exception("Failed to create encoder".into())?;

    let encoder_container = EncoderContainer {
        encoder,
        channels,
        mtu_size,
    };

    Ok(encoder_container.into_jlong_pointer())
}

fn get_encoder_container<'a>(
    env: &mut Env,
    encoder: &JObject
) -> Result<&'a mut EncoderContainer, JavaException> {
    let pointer = get_pointer_from_field(env, encoder)
        .err_into_opus_exception("Failed to get a pointer from the java object".into())?;

    Ok(unsafe { EncoderContainer::from_jlong_pointer(pointer) })
}

fn encoder_reset(
    env: &mut Env,
    encoder: JObject
) -> Result<(), JavaException> {
    let container = get_encoder_container(env, &encoder)?;

    container.encoder.reset_state()
        .err_into_opus_exception("Failed to reset encoder state".into())?;

    Ok(())
}

fn encoder_close(
    env: &mut Env,
    encoder: JObject
) -> Result<(), JavaException> {
    let pointer = get_pointer_from_field(env, &encoder)
        .err_into_opus_exception("Failed to get a pointer from the java object".into())?;

    let _container = unsafe { Box::from_raw(pointer as *mut EncoderContainer) };
    env.set_field(&encoder, jni_str!("pointer"), jni_sig!("J"), JValue::from(0 as jlong))
        .err_into_opus_exception("Failed to reset pointer".into())?;

    Ok(())
}

fn encoder_encode<'local>(
    env: &mut Env<'local>,
    encoder: JObject<'local>,
    samples: JShortArray<'local>
) -> Result<JByteArray<'local>, JavaException> {
    let container = get_encoder_container(env, &encoder)?;

    let samples_length = samples.len(env)
        .err_into_opus_exception("Failed to get samples array length".into())?;

    let mut samples_vec = vec![0i16 as jshort; samples_length];

    samples.get_region(env, 0, &mut samples_vec)
        .err_into_opus_exception("Failed to copy samples to rust vec".into())?;

    let result = container.encoder.encode_vec(&samples_vec, container.mtu_size as usize)
        .err_into_opus_exception("Failed to encode audio".into())?;

    let encoded_java = env.byte_array_from_slice(&result)
        .err_into_opus_exception("Failed to create java byte array".into())?;

    Ok(encoded_java)
}

fn encoder_set_bitrate(
    env: &mut Env,
    encoder: JObject,
    bitrate: jint
) -> Result<(), JavaException> {
    let container = get_encoder_container(env, &encoder)?;

    let bitrate = match bitrate {
        -1000 => Bitrate::Auto,
        -1 => Bitrate::Max,
        _ => {
            Bitrate::Bits(min(max(bitrate, 500), 512_000))
        }
    };

    container.encoder.set_bitrate(bitrate)
        .err_into_opus_exception("Failed to get encoder bitrate".into())?;

    Ok(())
}

fn encoder_get_bitrate(
    env: &mut Env,
    encoder: JObject
) -> Result<jint, JavaException> {
    let container = get_encoder_container(env, &encoder)?;

    let bitrate = container.encoder.get_bitrate()
        .err_into_opus_exception("Failed to get encoder bitrate".into())?;

    let bitrate = match bitrate {
        Bitrate::Auto => -1000,
        Bitrate::Max => -1,
        Bitrate::Bits(bits) => bits
    };

    Ok(bitrate)
}
