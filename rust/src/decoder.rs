use jni::objects::{JByteArray, JClass, JObject, JShortArray, JValue};
use jni::{jni_sig, jni_str, Env, EnvUnowned};
use jni::errors::ThrowRuntimeExAndDefault;
use jni::sys::{jboolean, jint, jlong};
use opus::{Channels, Decoder};
use crate::decoder_container::DecoderContainer;
use crate::util::exception::{JavaException, JavaExceptions};
use crate::util::into_exception::ErrIntoException;
use crate::util::pointer::{get_pointer_from_field, JavaPointers};

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusDecoder_createNative<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    sample_rate: jint,
    stereo: jboolean,
    frame_size: jint
) -> jlong {
    env.with_env(|env| create_decoder(sample_rate, stereo, frame_size).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusDecoder_resetNative<'local>(
    mut env: EnvUnowned<'local>,
    decoder: JObject<'local>
) {
    env.with_env(|env| decoder_reset(env, decoder).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusDecoder_closeNative<'local>(
    mut env: EnvUnowned<'local>,
    decoder: JObject<'local>
) {
    env.with_env(|env| decoder_close(env, decoder).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_com_plasmoverse_opus_OpusDecoder_decodeNative<'local>(
    mut env: EnvUnowned<'local>,
    decoder: JObject<'local>,
    encoded: JByteArray<'local>
) -> JShortArray<'local> {
    env.with_env(|env| decoder_decode(env, decoder, encoded).or_throw(env))
        .resolve::<ThrowRuntimeExAndDefault>()
}


fn create_decoder(
    sample_rate: jint,
    stereo: jboolean,
    frame_size: jint
) -> Result<jlong, JavaException> {
    let channels = if stereo { Channels::Stereo } else { Channels::Mono };

    let decoder = Decoder::new(sample_rate as u32, channels)
        .err_into_opus_exception("Failed to create decoder".into())?;

    let decoder_container = DecoderContainer {
        decoder,
        channels,
        frame_size,
    };

    Ok(decoder_container.into_jlong_pointer())
}

fn get_decoder_container<'a>(
    env: &mut Env,
    decoder: &JObject
) -> Result<&'a mut DecoderContainer, JavaException> {
    let pointer = get_pointer_from_field(env, decoder)
        .err_into_opus_exception("Failed to get a pointer from the java object".into())?;

    Ok(unsafe { DecoderContainer::from_jlong_pointer(pointer) })
}

fn decoder_reset(
    env: &mut Env,
    decoder: JObject
) -> Result<(), JavaException> {
    let container = get_decoder_container(env, &decoder)?;

    container.decoder.reset_state()
        .err_into_opus_exception("Failed to reset decoder state".into())?;

    Ok(())
}

fn decoder_close(
    env: &mut Env,
    decoder: JObject
) -> Result<(), JavaException> {
    let pointer = get_pointer_from_field(env, &decoder)
        .err_into_opus_exception("Failed to get a pointer from the java object".into())?;

    let _container = unsafe { Box::from_raw(pointer as *mut DecoderContainer) };
    env.set_field(&decoder, jni_str!("pointer"), jni_sig!("J"), JValue::from(0 as jlong))
        .err_into_opus_exception("Failed set reset pointer".into())?;

    Ok(())
}

fn decoder_decode<'local>(
    env: &mut Env<'local>,
    decoder: JObject<'local>,
    encoded: JByteArray<'local>
) -> Result<JShortArray<'local>, JavaException> {
    let container = get_decoder_container(env, &decoder)?;

    let encoded = match encoded.is_null() {
        true => vec![0u8; 0],
        false => env.convert_byte_array(encoded)
            .err_into_opus_exception("Failed to convert byte array to rust vec".into())?
    };

    let mut decoded = vec![0i16; container.frame_size as usize * container.channels as usize];

    let result = container.decoder.decode(&encoded, &mut decoded, false)
        .err_into_opus_exception("Failed to decode audio".into())?;

    let result_length = result * container.channels as usize;
    decoded.truncate(result_length);

    let decoded_java = env.new_short_array(result_length)
        .err_into_opus_exception("Failed to create java short array".into())?;

    decoded_java.set_region(env, 0, &decoded)
        .err_into_opus_exception("Failed to copy short vec into java short array".into())?;

    Ok(decoded_java)
}
