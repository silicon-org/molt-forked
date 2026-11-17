//! Limits on the nesting of a script and on the size of values.
//!
//! Without them a script could overflow the stack of its thread (deeply nested command
//! substitutions, parentheses or unary operators) or ask for an allocation the host cannot
//! make (`string repeat abc 1000000000000`); both abort the process. A limit raises a normal
//! Tcl error instead, with the error code `MOLT LIMIT NESTING` or `MOLT LIMIT SIZE`, which a
//! script can `catch` and an embedding application can recognize.
//!
//! Nesting at run time (nested evaluations, command substitutions and the operators of an
//! expression) counts against the recursion limit of the interpreter
//! ([`InterpBuilder::recursion_limit`](crate::InterpBuilder::recursion_limit)). The parser,
//! which reads a script before it is evaluated, bounds the nesting of command substitutions in
//! one text by [`MAX_PARSE_NESTING`]. The sizes follow the limits of Tcl 8.6, which keeps the
//! length of a string in an `int`.
//!
//! The stack a level takes depends on the build: about 10 KB in a debug build and about 1 KB
//! in a release build. A thread that runs scripts with the default recursion limit of 1000
//! needs a stack of about 16 MB in a debug build and 2 MB in a release build.

use crate::types::Exception;
use crate::value::Value;

/// The error code of a nesting limit.
pub const NESTING: &str = "MOLT LIMIT NESTING";

/// The error code of a size limit.
pub const SIZE: &str = "MOLT LIMIT SIZE";

/// The deepest nesting of command substitutions (`[...]`) in the text of one script.
pub const MAX_PARSE_NESTING: usize = 1000;

/// The longest string, in bytes: the largest `int`, as in Tcl 8.6.
pub const MAX_STRING_BYTES: usize = i32::MAX as usize;

/// The longest list: as many elements as pointers fit in the longest string.
pub const MAX_LIST_LENGTH: usize = MAX_STRING_BYTES / std::mem::size_of::<usize>();

/// The largest integer of an expression, in bits: as many bytes as the longest string.
pub const MAX_INTEGER_BITS: u64 = MAX_STRING_BYTES as u64 * 8;

/// A nesting error with `message`.
pub(crate) fn nesting_error(message: String) -> Exception {
    Exception::molt_err2(Value::from(NESTING), Value::from(message))
}

/// A size error with `message`.
pub(crate) fn size_error(message: String) -> Exception {
    Exception::molt_err2(Value::from(SIZE), Value::from(message))
}

/// Checks that `what` makes a string of `bytes` bytes (`None` if the count overflowed).
pub(crate) fn check_string(bytes: Option<usize>, what: &str) -> Result<usize, Exception> {
    match bytes {
        Some(n) if n <= MAX_STRING_BYTES => Ok(n),
        _ => Err(size_error(format!(
            "{what} makes a string longer than {MAX_STRING_BYTES} bytes"
        ))),
    }
}

/// Checks that `what` makes a list of `length` elements (`None` if the count overflowed).
pub(crate) fn check_list(length: Option<usize>, what: &str) -> Result<usize, Exception> {
    match length {
        Some(n) if n <= MAX_LIST_LENGTH => Ok(n),
        _ => Err(size_error(format!(
            "{what} makes a list longer than {MAX_LIST_LENGTH} elements"
        ))),
    }
}

/// Checks that an operator makes an integer of at most `bits` bits (`None` on overflow).
#[cfg(feature = "full")]
pub(crate) fn check_integer(bits: Option<u64>, what: &str) -> Result<(), Exception> {
    match bits {
        Some(n) if n <= MAX_INTEGER_BITS => Ok(()),
        _ => Err(size_error(format!(
            "{what} makes an integer longer than {MAX_INTEGER_BITS} bits"
        ))),
    }
}
