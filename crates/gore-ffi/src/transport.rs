use std::ffi::c_void;
use std::io::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::slice;
use std::str;

use serde_json::Value;

use crate::dispatch;

pub const TRANSPORT_ABI_V2: u32 = 2;
pub const TRANSPORT_STATUS_OK: u32 = 0;
pub const TRANSPORT_STATUS_INVALID_ARGUMENT: u32 = 1;
pub const TRANSPORT_STATUS_PANIC: u32 = 2;

pub const MAX_TRANSPORT_REQUEST_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TRANSPORT_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

const REQUEST_LIMIT_RESPONSE: &[u8] = br#"{"ok":false,"error":{"code":"FFI_REQUEST_LIMIT","message":"native request exceeds the 67108864-byte transport limit"}}"#;
const RESPONSE_LIMIT_RESPONSE: &[u8] = br#"{"ok":false,"error":{"code":"FFI_RESPONSE_LIMIT","message":"native response exceeds the 67108864-byte transport limit"}}"#;
const INVALID_UTF8_RESPONSE: &[u8] =
    br#"{"ok":false,"error":{"code":"BAD_REQUEST","message":"native request is not valid UTF-8"}}"#;
const SERIALIZE_RESPONSE: &[u8] =
    br#"{"ok":false,"error":{"code":"SERIALIZE","message":"response serialize failed"}}"#;

/// Length-aware response descriptor written by [`gore_core_execute_v2`]. `data` is borrowed from
/// the opaque `handle` and stays valid until that handle is released exactly once with
/// [`gore_core_response_free_v2`]. The bytes are UTF-8 JSON and are not NUL-terminated.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GoreCoreResponseV2 {
    pub data: *const u8,
    pub len: usize,
    pub handle: *mut c_void,
}

impl GoreCoreResponseV2 {
    const EMPTY: Self = Self {
        data: ptr::null(),
        len: 0,
        handle: ptr::null_mut(),
    };
}

struct OwnedResponse {
    bytes: Box<[u8]>,
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(8 * 1024)),
            limit,
            exceeded: false,
        }
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Some(remaining) = self.limit.checked_sub(self.bytes.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("transport response limit exceeded"));
        };
        if buf.len() > remaining {
            self.exceeded = true;
            return Err(io::Error::other("transport response limit exceeded"));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn serialize_value_bounded(value: &Value, limit: usize) -> Vec<u8> {
    let mut writer = BoundedWriter::new(limit);
    match serde_json::to_writer(&mut writer, value) {
        Ok(()) => writer.bytes,
        Err(_) if writer.exceeded => RESPONSE_LIMIT_RESPONSE.to_vec(),
        Err(_) => SERIALIZE_RESPONSE.to_vec(),
    }
}

/// Pure bounded serializer used by transport v2 and the in-process test seam.
pub(crate) fn execute_json_bounded(input: &str) -> Vec<u8> {
    serialize_value_bounded(&dispatch(input), MAX_TRANSPORT_RESPONSE_BYTES)
}

fn publish_response(out: *mut GoreCoreResponseV2, mut bytes: Vec<u8>) -> u32 {
    if bytes.is_empty() {
        bytes.extend_from_slice(SERIALIZE_RESPONSE);
    }
    let owned = Box::new(OwnedResponse {
        bytes: bytes.into_boxed_slice(),
    });
    let response = GoreCoreResponseV2 {
        data: owned.bytes.as_ptr(),
        len: owned.bytes.len(),
        handle: Box::into_raw(owned).cast::<c_void>(),
    };
    // SAFETY: the caller supplied a non-null writable out pointer, checked by the entry point.
    unsafe { out.write(response) };
    TRANSPORT_STATUS_OK
}

unsafe fn execute_v2_with<F>(
    request: *const u8,
    request_len: usize,
    out: *mut GoreCoreResponseV2,
    execute: F,
) -> u32
where
    F: FnOnce(&str) -> Vec<u8>,
{
    if out.is_null() {
        return TRANSPORT_STATUS_INVALID_ARGUMENT;
    }
    // Establish the failure invariant before inspecting any other argument or running core code.
    unsafe { out.write(GoreCoreResponseV2::EMPTY) };
    if request.is_null() && request_len != 0 {
        return TRANSPORT_STATUS_INVALID_ARGUMENT;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        // Check the claimed length before touching request memory. This also lets tests prove that
        // an oversized request cannot turn a one-byte pointer into an out-of-bounds read.
        if request_len > MAX_TRANSPORT_REQUEST_BYTES {
            return REQUEST_LIMIT_RESPONSE.to_vec();
        }
        let request_bytes = if request_len == 0 {
            &[]
        } else {
            // SAFETY: the ABI requires `request` to reference `request_len` readable bytes for the
            // duration of this call. Null/non-zero was rejected above.
            unsafe { slice::from_raw_parts(request, request_len) }
        };
        let Ok(input) = str::from_utf8(request_bytes) else {
            return INVALID_UTF8_RESPONSE.to_vec();
        };
        execute(input)
    }));

    match result {
        Ok(bytes) => publish_response(out, bytes),
        Err(_) => TRANSPORT_STATUS_PANIC,
    }
}

/// Exact transport-v2 capability probe. The versioned symbol name and value are both checked by
/// the Studio before any command is executed.
#[unsafe(no_mangle)]
pub extern "C" fn gore_core_transport_abi_v2() -> u32 {
    TRANSPORT_ABI_V2
}

/// Executes one bounded UTF-8 JSON request without relying on NUL termination.
///
/// Returns zero and a populated `out` for both successful commands and structured command errors.
/// Non-zero transport failures leave `out` entirely zeroed.
///
/// # Safety
/// `out` must be a valid writable response descriptor. `request` may be null only when
/// `request_len` is zero; otherwise it must reference that many readable bytes for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gore_core_execute_v2(
    request: *const u8,
    request_len: usize,
    out: *mut GoreCoreResponseV2,
) -> u32 {
    unsafe { execute_v2_with(request, request_len, out, execute_json_bounded) }
}

/// Releases an opaque response handle returned by [`gore_core_execute_v2`].
///
/// # Safety
/// `handle` must be null or an unreleased handle returned by a successful v2 call. Any other
/// pointer or a second release is undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gore_core_response_free_v2(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    // SAFETY: guaranteed by the ownership contract above. The boxed slice is freed with its exact
    // allocator metadata; the caller never has to echo a length or capacity to release it.
    unsafe { drop(Box::from_raw(handle.cast::<OwnedResponse>())) };
}

#[cfg(test)]
mod tests {
    use std::ptr;
    use std::slice;
    use std::thread;

    use serde_json::{json, Value};

    use super::*;

    unsafe fn invoke(request: *const u8, len: usize) -> (u32, GoreCoreResponseV2, Vec<u8>) {
        let mut out = GoreCoreResponseV2::EMPTY;
        let status = unsafe { gore_core_execute_v2(request, len, &mut out) };
        let bytes = if out.data.is_null() || out.len == 0 {
            Vec::new()
        } else {
            // SAFETY: a successful call owns these bytes until the returned handle is freed.
            unsafe { slice::from_raw_parts(out.data, out.len) }.to_vec()
        };
        (status, out, bytes)
    }

    fn value(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).expect("transport response must be JSON")
    }

    unsafe fn free(out: GoreCoreResponseV2) {
        unsafe { gore_core_response_free_v2(out.handle) };
    }

    #[test]
    fn probe_and_core_info_use_exact_length_without_nul() {
        assert_eq!(gore_core_transport_abi_v2(), TRANSPORT_ABI_V2);
        let request = br#"{"command":"core_info","payload":{}}"#;
        let mut storage = request.to_vec();
        storage.extend_from_slice(b"ignored trailing bytes");

        let (status, out, bytes) = unsafe { invoke(storage.as_ptr(), request.len()) };
        assert_eq!(status, TRANSPORT_STATUS_OK);
        assert!(!out.data.is_null());
        assert!(!out.handle.is_null());
        assert_eq!(out.len, bytes.len());
        let response = value(&bytes);
        assert_eq!(response["ok"], true);
        assert_eq!(response["abi"], crate::CORE_PROTOCOL_ABI);
        unsafe { free(out) };
    }

    #[test]
    fn invalid_utf8_and_embedded_nul_are_not_lossy_or_truncated() {
        let invalid_utf8 = [0xff];
        let (status, out, bytes) = unsafe { invoke(invalid_utf8.as_ptr(), invalid_utf8.len()) };
        assert_eq!(status, TRANSPORT_STATUS_OK);
        assert_eq!(value(&bytes)["error"]["code"], "BAD_REQUEST");
        unsafe { free(out) };

        let embedded_nul = b"{\"command\":\"core_info\"}\0ignored";
        let (status, out, bytes) = unsafe { invoke(embedded_nul.as_ptr(), embedded_nul.len()) };
        assert_eq!(status, TRANSPORT_STATUS_OK);
        assert_eq!(value(&bytes)["error"]["code"], "BAD_REQUEST");
        unsafe { free(out) };
    }

    #[test]
    fn argument_invariants_are_fail_closed_and_zeroed() {
        let mut out = GoreCoreResponseV2 {
            data: ptr::dangling(),
            len: 99,
            handle: ptr::dangling_mut(),
        };
        let status = unsafe { gore_core_execute_v2(ptr::null(), 1, &mut out) };
        assert_eq!(status, TRANSPORT_STATUS_INVALID_ARGUMENT);
        assert!(out.data.is_null());
        assert_eq!(out.len, 0);
        assert!(out.handle.is_null());

        let request = b"{}";
        let status =
            unsafe { gore_core_execute_v2(request.as_ptr(), request.len(), ptr::null_mut()) };
        assert_eq!(status, TRANSPORT_STATUS_INVALID_ARGUMENT);

        let (status, out, bytes) = unsafe { invoke(ptr::null(), 0) };
        assert_eq!(status, TRANSPORT_STATUS_OK);
        assert_eq!(value(&bytes)["error"]["code"], "BAD_REQUEST");
        unsafe { free(out) };
        unsafe { gore_core_response_free_v2(ptr::null_mut()) };
    }

    #[test]
    fn oversized_request_is_rejected_before_pointer_dereference() {
        let one_byte = 0u8;
        let (status, out, bytes) = unsafe {
            invoke(
                &one_byte,
                MAX_TRANSPORT_REQUEST_BYTES.checked_add(1).unwrap(),
            )
        };
        assert_eq!(status, TRANSPORT_STATUS_OK);
        assert_eq!(value(&bytes)["error"]["code"], "FFI_REQUEST_LIMIT");
        assert!(out.len < 1024);
        unsafe { free(out) };
    }

    #[test]
    fn bounded_serializer_replaces_oversized_response_with_small_error() {
        let oversized = json!({"data": "x".repeat(4096)});
        let bytes = serialize_value_bounded(&oversized, 8);
        assert_eq!(value(&bytes)["error"]["code"], "FFI_RESPONSE_LIMIT");
        assert!(bytes.len() < 1024);
    }

    #[test]
    fn panic_never_unwinds_across_the_abi_and_leaves_empty_output() {
        let request = b"{}";
        let mut out = GoreCoreResponseV2 {
            data: ptr::dangling(),
            len: 99,
            handle: ptr::dangling_mut(),
        };
        let status = unsafe {
            execute_v2_with(request.as_ptr(), request.len(), &mut out, |_| {
                panic!("transport test panic")
            })
        };
        assert_eq!(status, TRANSPORT_STATUS_PANIC);
        assert!(out.data.is_null());
        assert_eq!(out.len, 0);
        assert!(out.handle.is_null());
    }

    #[test]
    fn independent_buffers_can_be_freed_out_of_order_and_in_parallel() {
        let request = br#"{"command":"core_info","payload":{}}"#;
        let (first_status, first, first_bytes) = unsafe { invoke(request.as_ptr(), request.len()) };
        let (second_status, second, second_bytes) =
            unsafe { invoke(request.as_ptr(), request.len()) };
        assert_eq!(first_status, TRANSPORT_STATUS_OK);
        assert_eq!(second_status, TRANSPORT_STATUS_OK);
        assert_eq!(first_bytes, second_bytes);
        unsafe { free(second) };
        unsafe { free(first) };

        let joins: Vec<_> = (0..8)
            .map(|_| {
                thread::spawn(|| {
                    let request = br#"{"command":"core_info","payload":{}}"#;
                    let (status, out, bytes) = unsafe { invoke(request.as_ptr(), request.len()) };
                    let ok = status == TRANSPORT_STATUS_OK && value(&bytes)["ok"] == true;
                    unsafe { free(out) };
                    ok
                })
            })
            .collect();
        assert!(joins.into_iter().all(|join| join.join().unwrap()));
    }
}
