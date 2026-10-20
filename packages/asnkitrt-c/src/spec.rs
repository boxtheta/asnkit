// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Specifications: loading them, and encoding and decoding with them.
//!
//! What a file has to be to be loaded is set on an `asnkit_load_options`,
//! which is a handle and not a struct of C so that it can be given more
//! later. The options of encoding and decoding are few, and a struct.

use std::ffi::{CString, c_char, c_int, c_void};

use asnkitrt::container::{Binding, Checks, KEY_ID_SIZE, KEY_SIZE, Key, Nonce, Signature};
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

use crate::{
    ASNKIT_ERROR_ARGUMENT, ASNKIT_ERROR_IO, Result, bytes, failure, give_bytes, guard,
    guard_handle, handle, optional_text, out, text,
};

/// Tells if a nonce is accepted, with the context it was set with.
pub type NonceCallback = unsafe extern "C" fn(nonce: u32, context: *mut c_void) -> c_int;

enum NonceOption {
    None,
    Exact(u32),
    Accept(NonceCallback, *mut c_void),
    Any,
}

/// What a file has to be to be loaded, as `Checks`, and how.
pub struct LoadOptions {
    numeric_enums: bool,
    lenient: bool,
    nonce: NonceOption,
    bound_to: Option<String>,
    accept_any_binding: bool,
    verification_keys: Option<Vec<Key>>,
    accept_any_signature: bool,
    decryption_keys: Option<Vec<Key>>,
}

impl Default for LoadOptions {
    fn default() -> LoadOptions {
        LoadOptions {
            numeric_enums: false,
            lenient: false,
            nonce: NonceOption::None,
            bound_to: None,
            accept_any_binding: false,
            verification_keys: None,
            accept_any_signature: false,
            decryption_keys: None,
        }
    }
}

/// A specification loaded for a codec.
pub struct Spec {
    inner: Specification,
    codec: CString,
}

/// The options of encoding and decoding, as in the header.
#[repr(C)]
pub struct CodecOptions {
    /// The module of the type, or null if only one has a type of that
    /// name.
    pub module: *const c_char,
    pub check_constraints: c_int,
    /// The number of spaces for each level of JER and XER, or negative
    /// for none.
    pub indent: c_int,
}

fn codec_of(name: &str) -> Result<Codec> {
    Ok(match name {
        "ber" => Codec::Ber,
        "der" => Codec::Der,
        "oer" => Codec::Oer,
        "per" => Codec::Per,
        "uper" => Codec::Uper,
        "jer" => Codec::Jer,
        "xer" => Codec::Xer,
        "gser" => Codec::Gser,
        _ => {
            return Err(failure(
                ASNKIT_ERROR_ARGUMENT,
                format!(
                    "Unsupported codec '{name}': expected ber, der, oer, per, uper, jer, xer or gser."
                ),
            ));
        }
    })
}

/// The options of C, read: the module, if the constraints are checked
/// and the indent.
struct Read<'a> {
    module: Option<&'a str>,
    check_constraints: bool,
    indent: Option<usize>,
}

unsafe fn read_options<'a>(options: *const CodecOptions) -> Result<Read<'a>> {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return Ok(Read {
            module: None,
            check_constraints: false,
            indent: None,
        });
    };

    Ok(Read {
        module: unsafe { optional_text(options.module, "the module as text") }?,
        check_constraints: options.check_constraints != 0,
        indent: usize::try_from(options.indent).ok(),
    })
}

unsafe fn key(id: *const u8, key: *const u8) -> Result<Key> {
    let id = unsafe { bytes(id, KEY_ID_SIZE, "the id of the key") }?;
    let key = unsafe { bytes(key, KEY_SIZE, "the key") }?;

    Ok(Key {
        id: id.try_into().expect("KEY_ID_SIZE bytes"),
        key: key.try_into().expect("KEY_SIZE bytes"),
    })
}

unsafe fn options_mut<'a>(options: *mut LoadOptions) -> Result<&'a mut LoadOptions> {
    unsafe { options.as_mut() }
        .ok_or_else(|| failure(ASNKIT_ERROR_ARGUMENT, "Expected the load options."))
}

/// New load options, that load only a file that is not signed, not
/// encrypted, with no nonce and not bound.
#[unsafe(no_mangle)]
pub extern "C" fn asnkit_load_options_new() -> *mut LoadOptions {
    guard_handle(|| Ok(Box::into_raw(Box::default())))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_free(options: *mut LoadOptions) {
    if !options.is_null() {
        drop(unsafe { Box::from_raw(options) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_numeric_enums(
    options: *mut LoadOptions,
    numeric_enums: c_int,
) -> c_int {
    guard(|| {
        unsafe { options_mut(options) }?.numeric_enums = numeric_enums != 0;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_lenient(
    options: *mut LoadOptions,
    lenient: c_int,
) -> c_int {
    guard(|| {
        unsafe { options_mut(options) }?.lenient = lenient != 0;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_nonce(
    options: *mut LoadOptions,
    nonce: u32,
) -> c_int {
    guard(|| {
        unsafe { options_mut(options) }?.nonce = NonceOption::Exact(nonce);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_nonce_callback(
    options: *mut LoadOptions,
    accept: Option<NonceCallback>,
    context: *mut c_void,
) -> c_int {
    guard(|| {
        let accept =
            accept.ok_or_else(|| failure(ASNKIT_ERROR_ARGUMENT, "Expected a function."))?;

        unsafe { options_mut(options) }?.nonce = NonceOption::Accept(accept, context);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_accept_any_nonce(
    options: *mut LoadOptions,
) -> c_int {
    guard(|| {
        unsafe { options_mut(options) }?.nonce = NonceOption::Any;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_bound_to(
    options: *mut LoadOptions,
    bound_to: *const c_char,
) -> c_int {
    guard(|| {
        let bound_to = unsafe { text(bound_to, "what the file is bound to") }?;

        unsafe { options_mut(options) }?.bound_to = Some(bound_to.into());
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_accept_any_binding(
    options: *mut LoadOptions,
) -> c_int {
    guard(|| {
        unsafe { options_mut(options) }?.accept_any_binding = true;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_add_verification_key(
    options: *mut LoadOptions,
    id: *const u8,
    public_key: *const u8,
) -> c_int {
    guard(|| {
        let key = unsafe { key(id, public_key) }?;

        unsafe { options_mut(options) }?
            .verification_keys
            .get_or_insert_with(Vec::new)
            .push(key);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_set_accept_any_signature(
    options: *mut LoadOptions,
) -> c_int {
    guard(|| {
        unsafe { options_mut(options) }?.accept_any_signature = true;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_load_options_add_decryption_key(
    options: *mut LoadOptions,
    id: *const u8,
    secret_key: *const u8,
) -> c_int {
    guard(|| {
        let key = unsafe { key(id, secret_key) }?;

        unsafe { options_mut(options) }?
            .decryption_keys
            .get_or_insert_with(Vec::new)
            .push(key);
        Ok(())
    })
}

fn load(asni: &[u8], codec: Option<&str>, options: &LoadOptions) -> Result<Spec> {
    let codec = codec.unwrap_or("ber");
    let mut load = Options::new(codec_of(codec)?);

    load.numeric_enums = options.numeric_enums;
    load.lenient = options.lenient;

    let accept_nonce = |nonce: u32| match options.nonce {
        NonceOption::Accept(accept, context) => unsafe { accept(nonce, context) != 0 },
        _ => false,
    };
    let checks = Checks {
        signature: match &options.verification_keys {
            Some(keys) => Signature::Keys(keys),
            None if options.accept_any_signature => Signature::Any,
            None => Signature::None,
        },
        nonce: match options.nonce {
            NonceOption::None => Nonce::None,
            NonceOption::Exact(nonce) => Nonce::Exact(nonce),
            NonceOption::Accept(..) => Nonce::Accept(&accept_nonce),
            NonceOption::Any => Nonce::Any,
        },
        decryption_keys: options.decryption_keys.as_deref(),
        bound_to: match &options.bound_to {
            Some(bound_to) => Binding::To(bound_to),
            None if options.accept_any_binding => Binding::Any,
            None => Binding::None,
        },
        ..Checks::default()
    };

    Ok(Spec {
        inner: Specification::from_asni(asni, &checks, load)?,
        codec: CString::new(codec).expect("a codec has no NUL"),
    })
}

/// Loads the `length` bytes of an `.asni` file at `asni` for `codec`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_spec_load(
    asni: *const u8,
    length: usize,
    codec: *const c_char,
    options: *const LoadOptions,
    spec: *mut *mut Spec,
) -> c_int {
    guard(|| {
        let spec = unsafe { out(spec, "the specification") }?;
        let asni = unsafe { bytes(asni, length, "the .asni file") }?;
        let codec = unsafe { optional_text(codec, "the codec as text") }?;
        let default = LoadOptions::default();
        let options = unsafe { options.as_ref() }.unwrap_or(&default);

        *spec = Box::into_raw(Box::new(load(asni, codec, options)?));
        Ok(())
    })
}

/// Reads the `.asni` file at `path` and loads it for `codec`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_spec_load_file(
    path: *const c_char,
    codec: *const c_char,
    options: *const LoadOptions,
    spec: *mut *mut Spec,
) -> c_int {
    guard(|| {
        let spec = unsafe { out(spec, "the specification") }?;

        if path.is_null() {
            return Err(failure(ASNKIT_ERROR_ARGUMENT, "Expected a path."));
        }

        let path = unsafe { std::ffi::CStr::from_ptr(path) };
        // A path is bytes where it is, and has to be UTF-8 elsewhere.
        #[cfg(unix)]
        let path = std::path::Path::new(
            <std::ffi::OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(path.to_bytes()),
        );
        #[cfg(not(unix))]
        let path = std::path::Path::new(
            path.to_str()
                .map_err(|_| failure(ASNKIT_ERROR_ARGUMENT, "Expected the path in UTF-8."))?,
        );
        let asni = std::fs::read(path).map_err(|error| {
            failure(
                ASNKIT_ERROR_IO,
                format!("Can not read {}: {error}", path.display()),
            )
        })?;
        let codec = unsafe { optional_text(codec, "the codec as text") }?;
        let default = LoadOptions::default();
        let options = unsafe { options.as_ref() }.unwrap_or(&default);

        *spec = Box::into_raw(Box::new(load(&asni, codec, options)?));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_spec_free(spec: *mut Spec) {
    if !spec.is_null() {
        drop(unsafe { Box::from_raw(spec) });
    }
}

/// The codec the specification is loaded for, as `ber`, or null if
/// `spec` is.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_spec_codec(spec: *const Spec) -> *const c_char {
    match unsafe { spec.as_ref() } {
        Some(spec) => spec.codec.as_ptr(),
        None => std::ptr::null(),
    }
}

fn check(spec: &Spec, name: &str, value: &Value, options: &Read) -> Result<()> {
    Ok(match options.module {
        Some(module) => spec.inner.check_constraints_in(module, name, value),
        None => spec.inner.check_constraints(name, value),
    }?)
}

/// Encodes `value` as the type `name`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_encode(
    spec: *const Spec,
    name: *const c_char,
    value: *const Value<'static>,
    options: *const CodecOptions,
    data: *mut *mut u8,
    length: *mut usize,
) -> c_int {
    guard(|| {
        let spec = unsafe { handle(spec, "a specification") }?;
        let name = unsafe { text(name, "the name of the type") }?;
        let value = unsafe { handle(value, "a value") }?;
        let options = unsafe { read_options(options) }?;
        let data = unsafe { out(data, "the data") }?;
        let length = unsafe { out(length, "the length") }?;

        if options.check_constraints {
            check(spec, name, value, &options)?;
        }

        let encoded = match (options.module, options.indent) {
            (Some(module), None) => spec.inner.encode_in(module, name, value),
            (None, None) => spec.inner.encode(name, value),
            (Some(module), Some(indent)) => {
                spec.inner.encode_in_indented(module, name, value, indent)
            }
            (None, Some(indent)) => spec.inner.encode_indented(name, value, indent),
        }?;

        give_bytes(encoded, data, length);
        Ok(())
    })
}

/// Decodes the `length` bytes at `data` as the type `name`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_decode(
    spec: *const Spec,
    name: *const c_char,
    data: *const u8,
    length: usize,
    options: *const CodecOptions,
    value: *mut *mut Value<'static>,
) -> c_int {
    guard(|| {
        let spec = unsafe { handle(spec, "a specification") }?;
        let name = unsafe { text(name, "the name of the type") }?;
        let data = unsafe { bytes(data, length, "the data") }?;
        let options = unsafe { read_options(options) }?;
        let value = unsafe { out(value, "the value") }?;
        let decoded = match options.module {
            Some(module) => spec.inner.decode_in(module, name, data),
            None => spec.inner.decode(name, data),
        }?;

        if options.check_constraints {
            check(spec, name, &decoded, &options)?;
        }

        // C keeps the value past the specification.
        *value = Box::into_raw(Box::new(decoded.into_owned()));
        Ok(())
    })
}

/// Decodes the first value of `data`, and how many bytes it took. BER
/// and DER only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_decode_with_length(
    spec: *const Spec,
    name: *const c_char,
    data: *const u8,
    length: usize,
    options: *const CodecOptions,
    value: *mut *mut Value<'static>,
    consumed: *mut usize,
) -> c_int {
    guard(|| {
        let spec = unsafe { handle(spec, "a specification") }?;
        let name = unsafe { text(name, "the name of the type") }?;
        let data = unsafe { bytes(data, length, "the data") }?;
        let options = unsafe { read_options(options) }?;
        let value = unsafe { out(value, "the value") }?;
        let consumed = unsafe { out(consumed, "the length") }?;

        if options.module.is_some() {
            return Err(failure(
                ASNKIT_ERROR_ARGUMENT,
                "asnkit_decode_with_length() does not take a module.",
            ));
        }

        let (decoded, used) = spec.inner.decode_with_length(name, data)?;

        if options.check_constraints {
            check(spec, name, &decoded, &options)?;
        }

        // C keeps the value past the specification.
        *value = Box::into_raw(Box::new(decoded.into_owned()));
        *consumed = used;
        Ok(())
    })
}

/// The length of the first value of `data`, with `known` 0 if there is
/// not enough data to tell. BER and DER only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_decode_length(
    spec: *const Spec,
    data: *const u8,
    length: usize,
    value_length: *mut usize,
    known: *mut c_int,
) -> c_int {
    guard(|| {
        let spec = unsafe { handle(spec, "a specification") }?;
        let data = unsafe { bytes(data, length, "the data") }?;
        let value_length = unsafe { out(value_length, "the length") }?;
        let known = unsafe { out(known, "if the length is known") }?;

        match spec.inner.decode_length(data)? {
            Some(found) => {
                *value_length = found;
                *known = 1;
            }
            None => {
                *value_length = 0;
                *known = 0;
            }
        }

        Ok(())
    })
}

/// Checks `value` against the constraints of the type `name`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_check_constraints(
    spec: *const Spec,
    name: *const c_char,
    value: *const Value<'static>,
    options: *const CodecOptions,
) -> c_int {
    guard(|| {
        let spec = unsafe { handle(spec, "a specification") }?;
        let name = unsafe { text(name, "the name of the type") }?;
        let value = unsafe { handle(value, "a value") }?;
        let options = unsafe { read_options(options) }?;

        check(spec, name, value, &options)
    })
}
