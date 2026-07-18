//! # FFI (Foreign Function Interface) for `arctgz`
//!
//! This module exposes a C-compatible API for the `arctgz` library, allowing
//! integration with code written in C or other languages that can call C
//! functions.
//!
//! ## Overview
//!
//! The FFI surface consists of:
//!
//! - A C-compatible error code enumeration [`ArctgzErrorCode`], which maps
//!   internal errors to simple integers.
//! - Two entry points:
//!     - [`arctgz_init`] – initializes a new project.
//!     - [`arctgz_last_error_message`] – retrieves a human-readable error
//!       message after a failure.
//!
//! Error messages are stored in a thread-local variable, so they are safe to
//! use in multi‑threaded environments (each thread gets its own error state).
//!
//! ## Thread safety
//!
//! The last error message is stored per thread. Calling `arctgz_init` from
//! multiple threads simultaneously is safe, provided each call uses its own
//! thread (the standard Rust thread model). The functions are not marked as
//! `Sync`; callers must ensure they are only used from one thread at a time
//! for a given call, which is naturally the case with direct C calls.
//!
//! ## Pointer lifetimes
//!
//! Both functions accept or return raw C string pointers (`*const c_char`).
//! Callers must respect the safety preconditions documented for each function.
//! In particular, the pointer returned by [`arctgz_last_error_message`] is
//! valid only until the next FFI call on the same thread that might update
//! the error state. If the caller needs the message beyond that point, they
//! must copy it (e.g., with `strdup` or equivalent).

use crate::core::init::init;
use crate::handler::ArctgzError;
use libc::{c_char, c_int};
use std::ffi::{CStr, CString};
use std::panic;
use std::path::Path;

/// C-compatible error codes returned by FFI functions.
///
/// Each variant corresponds to a possible failure of [`arctgz_init`].
/// The value `0` always indicates success.
///
/// # Mapping from internal errors
///
/// This enum is generated from [`ArctgzError`] via the
/// [`From<&ArctgzError>`] implementation.
///
/// # Variants
///
/// - `Success` - Operation completed successfully.
/// - `IoError` - A filesystem I/O error occurred (e.g., creating directories,
///   writing files).
/// - `JsonError` - JSON serialization or deserialization failed.
/// - `AlreadyInitialized` - The project was already initialized (i.e., an
///   `arctgz.init` file already exists).
/// - `InvalidPath` - The provided path is empty, contains invalid UTF-8, or
///   is `NULL`.
/// - `PathNotAllowed` - The path is not under the user's home directory.
/// - `DirectoryNotEmpty` - The target directory exists and is not empty,
///   and `force` was not set.
/// - `ConfigValidation` - The generated configuration failed validation
///   (should not normally happen with default config).
/// - `UnknownError` - A panic was caught inside the FFI function; indicates
///   an internal bug.
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum ArctgzErrorCode {
    Success = 0,
    IoError = 1,
    JsonError = 2,
    AlreadyInitialized = 3,
    InvalidPath = 4,
    PathNotAllowed = 5,
    DirectoryNotEmpty = 6,
    ConfigValidation = 7,
    UnknownError = 99,
}

/// Converts a reference to an internal [`ArctgzError`] into an
/// [`ArctgzErrorCode`].
///
/// This allows the FFI layer to translate typed errors into simple codes
/// that C callers can inspect.
impl From<&ArctgzError> for ArctgzErrorCode {
    fn from(err: &ArctgzError) -> Self {
        match err {
            ArctgzError::Io(_) => ArctgzErrorCode::IoError,
            ArctgzError::Json(_) => ArctgzErrorCode::JsonError,
            ArctgzError::AlreadyInitialized => ArctgzErrorCode::AlreadyInitialized,
            ArctgzError::InvalidPath(_) => ArctgzErrorCode::InvalidPath,
            ArctgzError::PathNotAllowed(_) => ArctgzErrorCode::PathNotAllowed,
            ArctgzError::DirectoryNotEmpty(_) => ArctgzErrorCode::DirectoryNotEmpty,
            ArctgzError::ConfigValidation(_) => ArctgzErrorCode::ConfigValidation,
        }
    }
}

// ---------------------------------------------------------------------------
// Thread‑local error storage
// ---------------------------------------------------------------------------
thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

/// Saves a human‑readable error message that can later be retrieved by
/// [`arctgz_last_error_message`].
///
/// If `msg` contains an interior null byte, a fallback string is stored
/// instead.
fn set_last_error(msg: &str) {
    if let Ok(c_msg) = CString::new(msg) {
        LAST_ERROR.with(|cell| {
            *cell.borrow_mut() = Some(c_msg);
        });
    } else {
        let fallback = CString::new("Error message contains invalid null byte").unwrap();
        LAST_ERROR.with(|cell| {
            *cell.borrow_mut() = Some(fallback);
        });
    }
}

// ---------------------------------------------------------------------------
// Public FFI functions
// ---------------------------------------------------------------------------

/// Returns a pointer to the last error message for the current thread.
///
/// If no error has occurred (or the last call was successful), returns a
/// null pointer.
///
/// # Safety
///
/// The returned pointer borrows from a thread‑local `CString`. The pointed‑to
/// memory is valid **only** until the next call to any FFI function on the
/// same thread that might update the error state (including another call to
/// `arctgz_init`). If the caller needs the message beyond that, they must
/// copy it (e.g., via `strdup`).
///
/// The pointer must not be freed by the caller; it is managed internally.
#[unsafe(no_mangle)]
pub extern "C" fn arctgz_last_error_message() -> *const c_char {
    LAST_ERROR.with(|cell| match cell.borrow().as_ref() {
        Some(cs) => cs.as_ptr(),
        None => std::ptr::null(),
    })
}

/// Initializes a new Arctgz project at the given path.
///
/// Creates the project directory (if it does not exist) and writes a default
/// configuration file (`arctgz.init`) into it. The directory and all parent
/// directories will be created as needed, provided the final canonical path
/// lies under the current user’s home directory.
///
/// If `force` is non‑zero, the initialization proceeds even if the target
/// directory already exists and is non‑empty; otherwise an error is returned
/// for non‑empty directories.
///
/// # Safety
///
/// `project_path` must be a valid, non‑dangling pointer to a null‑terminated
/// C string containing valid UTF‑8. It is dereferenced in the FFI call and
/// must remain valid for the duration of the function (the call does not store
/// the pointer). Passing a null pointer results in an `InvalidPath` error.
///
/// # Panics
///
/// This function catches any unwinding panics from the internal Rust code and
/// returns `UnknownError`. The caller will not observe a Rust panic across
/// the FFI boundary.
///
/// # Error retrieval
///
/// If this function returns a code other than `Success`, the caller may obtain
/// a human‑readable explanation via [`arctgz_last_error_message`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arctgz_init(project_path: *const c_char, force: c_int) -> ArctgzErrorCode {
    let result = panic::catch_unwind(|| {
        if project_path.is_null() {
            set_last_error("Null pointer passed as project_path");
            return ArctgzErrorCode::InvalidPath;
        }

        // SAFETY: caller guarantees `project_path` is a valid null‑terminated string.
        let c_str = unsafe { CStr::from_ptr(project_path) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => {
                set_last_error("Project path is not valid UTF-8");
                return ArctgzErrorCode::InvalidPath;
            }
        };

        let force_bool = force != 0;

        match init(Path::new(path_str), force_bool) {
            Ok(()) => {
                LAST_ERROR.with(|cell| *cell.borrow_mut() = None);
                ArctgzErrorCode::Success
            }
            Err(e) => {
                set_last_error(&e.to_string());
                ArctgzErrorCode::from(&e)
            }
        }
    });

    match result {
        Ok(code) => code,
        Err(_) => {
            set_last_error("Internal error: unexpected panic in init");
            ArctgzErrorCode::UnknownError
        }
    }
}
