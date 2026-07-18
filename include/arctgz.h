#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

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
enum class ArctgzErrorCode {
  Success = 0,
  IoError = 1,
  JsonError = 2,
  AlreadyInitialized = 3,
  InvalidPath = 4,
  PathNotAllowed = 5,
  DirectoryNotEmpty = 6,
  ConfigValidation = 7,
  UnknownError = 99,
};

extern "C" {

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
const char *arctgz_last_error_message();

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
ArctgzErrorCode arctgz_init(const char *project_path, int force);

}  // extern "C"
