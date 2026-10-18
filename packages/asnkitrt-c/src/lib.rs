// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The runtime of asnkit for C: the runtime for Rust, behind functions
//! of C that `include/asnkitrt.h` declares and documents.
//!
//! It loads `.asni` files and encodes and decodes with them, as the
//! runtime for WebAssembly does (see `spec`), with values that are
//! handles to the values of the runtime (see `value`).
//!
//! Every function returns a status, or a handle that is null on an
//! error, and leaves the message of an error for
//! `asnkit_last_error()`. No panic gets out of a function: one is caught
//! and returned as `ASNKIT_ERROR_PANIC`. The runtime has
//! `#![forbid(unsafe_code)]`, which is why this is a crate of its own.

// What is safe to pass to each function is in the header, for C, where
// it is read.
#![allow(clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub mod spec;
pub mod value;

pub const ASNKIT_OK: c_int = 0;
/// The `.asni` file could not be read, or was not what the checks
/// expect.
pub const ASNKIT_ERROR_LOAD: c_int = 1;
pub const ASNKIT_ERROR_ENCODE: c_int = 2;
pub const ASNKIT_ERROR_DECODE: c_int = 3;
pub const ASNKIT_ERROR_CONSTRAINTS: c_int = 4;
pub const ASNKIT_ERROR_UNSUPPORTED: c_int = 5;
pub const ASNKIT_ERROR_NO_SUCH_TYPE: c_int = 6;
/// A null pointer, text that is not UTF-8, an unknown codec and so on.
pub const ASNKIT_ERROR_ARGUMENT: c_int = 7;
/// A value that is not of the kind asked for.
pub const ASNKIT_ERROR_KIND: c_int = 8;
/// A number that does not fit, or a buffer that is too small.
pub const ASNKIT_ERROR_RANGE: c_int = 9;
/// A file that could not be read.
pub const ASNKIT_ERROR_IO: c_int = 10;
/// A bug of the runtime, caught before it got to C.
pub const ASNKIT_ERROR_PANIC: c_int = 11;

/// Why a function failed, before it is left for `asnkit_last_error()`.
pub(crate) struct Failure {
    status: c_int,
    message: String,
    /// Of a decoding error.
    offset: Option<usize>,
}

pub(crate) type Result<T> = std::result::Result<T, Failure>;

pub(crate) fn failure(status: c_int, message: impl Into<String>) -> Failure {
    Failure {
        status,
        message: message.into(),
        offset: None,
    }
}

impl From<asnkitrt::Error> for Failure {
    fn from(error: asnkitrt::Error) -> Failure {
        use asnkitrt::Error::*;

        let message = error.to_string();
        let (status, offset) = match error {
            Encode(..) => (ASNKIT_ERROR_ENCODE, None),
            Decode(_, offset, _) => (ASNKIT_ERROR_DECODE, Some(offset)),
            Constraints(..) => (ASNKIT_ERROR_CONSTRAINTS, None),
            Unsupported(..) => (ASNKIT_ERROR_UNSUPPORTED, None),
            NoSuchType => (ASNKIT_ERROR_NO_SUCH_TYPE, None),
            // Everything else is about the file.
            _ => (ASNKIT_ERROR_LOAD, None),
        };

        Failure {
            status,
            message,
            offset,
        }
    }
}

/// The last error of this thread: its message and its offset.
struct LastError {
    message: CString,
    offset: Option<usize>,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<LastError>> = const { RefCell::new(None) };
}

fn remember(failure: Failure) -> c_int {
    // A message of the runtime has no NUL, but one of a file name may.
    let message = CString::new(failure.message.replace('\0', "?")).unwrap_or_default();

    LAST_ERROR.with(|last| {
        *last.borrow_mut() = Some(LastError {
            message,
            offset: failure.offset,
        })
    });

    failure.status
}

fn panicked() -> Failure {
    failure(
        ASNKIT_ERROR_PANIC,
        "The runtime panicked, which is a bug; please report it.",
    )
}

/// Runs the body of a function that returns a status.
pub(crate) fn guard(body: impl FnOnce() -> Result<()>) -> c_int {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(())) => ASNKIT_OK,
        Ok(Err(failure)) => remember(failure),
        Err(_) => remember(panicked()),
    }
}

/// Runs the body of a function that returns a handle, null on an error.
pub(crate) fn guard_handle<T>(body: impl FnOnce() -> Result<*mut T>) -> *mut T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(handle)) => handle,
        Ok(Err(failure)) => {
            remember(failure);
            std::ptr::null_mut()
        }
        Err(_) => {
            remember(panicked());
            std::ptr::null_mut()
        }
    }
}

/// A text of C that has to be given.
pub(crate) unsafe fn text<'a>(text: *const c_char, what: &str) -> Result<&'a str> {
    if text.is_null() {
        return Err(failure(ASNKIT_ERROR_ARGUMENT, format!("Expected {what}.")));
    }

    unsafe { CStr::from_ptr(text) }
        .to_str()
        .map_err(|_| failure(ASNKIT_ERROR_ARGUMENT, format!("Expected {what} in UTF-8.")))
}

/// A text of C that may be null.
pub(crate) unsafe fn optional_text<'a>(
    value: *const c_char,
    what: &str,
) -> Result<Option<&'a str>> {
    if value.is_null() {
        Ok(None)
    } else {
        unsafe { text(value, what) }.map(Some)
    }
}

/// `length` bytes, which may be null if there are none.
pub(crate) unsafe fn bytes<'a>(data: *const u8, length: usize, what: &str) -> Result<&'a [u8]> {
    if length == 0 {
        return Ok(&[]);
    }

    if data.is_null() {
        return Err(failure(ASNKIT_ERROR_ARGUMENT, format!("Expected {what}.")));
    }

    Ok(unsafe { std::slice::from_raw_parts(data, length) })
}

/// A pointer to write a result to.
pub(crate) unsafe fn out<'a, T>(pointer: *mut T, what: &str) -> Result<&'a mut T> {
    unsafe { pointer.as_mut() }.ok_or_else(|| {
        failure(
            ASNKIT_ERROR_ARGUMENT,
            format!("Expected a place for {what}."),
        )
    })
}

/// A handle that has to be given.
pub(crate) unsafe fn handle<'a, T>(pointer: *const T, what: &str) -> Result<&'a T> {
    unsafe { pointer.as_ref() }
        .ok_or_else(|| failure(ASNKIT_ERROR_ARGUMENT, format!("Expected {what}.")))
}

/// The message of the last error of this thread, or "" if there was
/// none. It is valid until the next error of the thread.
#[unsafe(no_mangle)]
pub extern "C" fn asnkit_last_error() -> *const c_char {
    LAST_ERROR.with(|last| match &*last.borrow() {
        Some(last) => last.message.as_ptr(),
        None => c"".as_ptr(),
    })
}

/// Writes the offset of the last error of this thread to `offset` and
/// returns 1 if it was an error of decoding, and returns 0 if not.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_last_error_offset(offset: *mut usize) -> c_int {
    let found = LAST_ERROR.with(|last| last.borrow().as_ref().and_then(|last| last.offset));

    match (found, unsafe { offset.as_mut() }) {
        (Some(found), Some(offset)) => {
            *offset = found;
            1
        }
        _ => 0,
    }
}

/// The version of the runtime, as `1.2.3`.
#[unsafe(no_mangle)]
pub extern "C" fn asnkit_version() -> *const c_char {
    const VERSION: &CStr =
        match CStr::from_bytes_with_nul(concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes()) {
            Ok(version) => version,
            Err(_) => panic!("The version has no NUL."),
        };

    VERSION.as_ptr()
}

/// A buffer for C, followed by a NUL that `length` does not count, so
/// that JER and XER are also a string of C.
pub(crate) fn give_bytes(mut data: Vec<u8>, out: &mut *mut u8, length: &mut usize) {
    *length = data.len();
    data.push(0);
    *out = Box::into_raw(data.into_boxed_slice()).cast();
}

/// Frees a buffer of `length` bytes that a function of the runtime
/// gave. Nothing is done for null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_bytes_free(data: *mut u8, length: usize) {
    if !data.is_null() {
        let _ = catch_unwind(|| {
            // With the NUL that follows.
            let slice = std::ptr::slice_from_raw_parts_mut(data, length + 1);

            drop(unsafe { Box::from_raw(slice) });
        });
    }
}
