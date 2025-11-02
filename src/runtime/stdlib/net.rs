use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU64, Ordering};

use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

// ============================================================================
// Networking - TCP and HTTP
// Simplified implementation using std::net
// ============================================================================

type HandleId = u64;
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

fn next_handle_id() -> HandleId {
    NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst)
}

// Connection handle
struct Connection {
    _id: HandleId,
    // For now, just store address
    // Full implementation would maintain actual TCP connections
    _address: String,
}

// Listener handle
struct Listener {
    _id: HandleId,
    _address: String,
}

// HTTP Response
struct HttpResponse {
    status: i32,
    body: String,
    _headers: String,
}

static CONNECTIONS: Lazy<RwLock<std::collections::HashMap<HandleId, Connection>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

static LISTENERS: Lazy<RwLock<std::collections::HashMap<HandleId, Listener>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

static HTTP_RESPONSES: Lazy<RwLock<std::collections::HashMap<HandleId, HttpResponse>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

#[no_mangle]
pub extern "C" fn otter_std_net_listen(addr: *const c_char) -> u64 {
    if addr.is_null() {
        return 0;
    }

    let address = unsafe { CStr::from_ptr(addr).to_str().unwrap_or("").to_string() };

    let id = next_handle_id();
    let listener = Listener {
        _id: id,
        _address: address,
    };

    LISTENERS.write().insert(id, listener);
    id
}

#[no_mangle]
pub extern "C" fn otter_std_net_dial(addr: *const c_char) -> u64 {
    if addr.is_null() {
        return 0;
    }

    let address = unsafe { CStr::from_ptr(addr).to_str().unwrap_or("").to_string() };

    let id = next_handle_id();
    let conn = Connection {
        _id: id,
        _address: address,
    };

    CONNECTIONS.write().insert(id, conn);
    id
}

#[no_mangle]
pub extern "C" fn otter_std_net_send(conn: u64, data: *const c_char) -> i32 {
    if data.is_null() {
        return 0;
    }

    let _data_str = unsafe { CStr::from_ptr(data).to_str().unwrap_or("").to_string() };

    let connections = CONNECTIONS.read();
    if connections.contains_key(&conn) {
        // In full implementation, would send data over TCP connection
        // For now, just return success
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_std_net_recv(conn: u64) -> *mut c_char {
    let connections = CONNECTIONS.read();
    if connections.contains_key(&conn) {
        // In full implementation, would receive data from TCP connection
        // For now, return empty string
        CString::new("")
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn otter_std_net_close(conn: u64) {
    CONNECTIONS.write().remove(&conn);
}

#[no_mangle]
pub extern "C" fn otter_std_net_http_get(url: *const c_char) -> u64 {
    if url.is_null() {
        return 0;
    }

    let url_str = unsafe { CStr::from_ptr(url).to_str().unwrap_or("").to_string() };

    let id = next_handle_id();

    // Simple HTTP GET using std::net (blocking)
    // Full implementation would use reqwest or similar
    let response = match std::net::TcpStream::connect(
        &url_str
            .replace("http://", "")
            .replace("https://", "")
            .split('/')
            .next()
            .unwrap_or(""),
    ) {
        Ok(_) => {
            // Simplified - in real implementation would parse HTTP response
            HttpResponse {
                status: 200,
                body: format!("Response from {}", url_str),
                _headers: "Content-Type: text/plain".to_string(),
            }
        }
        Err(_) => HttpResponse {
            status: 500,
            body: "Connection failed".to_string(),
            _headers: String::new(),
        },
    };

    HTTP_RESPONSES.write().insert(id, response);
    id
}

#[no_mangle]
pub extern "C" fn otter_std_net_http_post(url: *const c_char, body: *const c_char) -> u64 {
    if url.is_null() {
        return 0;
    }

    let url_str = unsafe { CStr::from_ptr(url).to_str().unwrap_or("").to_string() };

    let body_str = if body.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(body).to_str().unwrap_or("").to_string() }
    };

    let id = next_handle_id();

    // Simple HTTP POST (simplified)
    let response = HttpResponse {
        status: 200,
        body: format!("POST response for {}: {}", url_str, body_str),
        _headers: "Content-Type: text/plain".to_string(),
    };

    HTTP_RESPONSES.write().insert(id, response);
    id
}

#[no_mangle]
pub extern "C" fn otter_std_net_response_status(response: u64) -> i32 {
    let responses = HTTP_RESPONSES.read();
    if let Some(resp) = responses.get(&response) {
        resp.status
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_std_net_response_body(response: u64) -> *mut c_char {
    let responses = HTTP_RESPONSES.read();
    if let Some(resp) = responses.get(&response) {
        CString::new(resp.body.clone())
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    } else {
        std::ptr::null_mut()
    }
}

fn register_std_net_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "net.listen".into(),
        symbol: "otter_std_net_listen".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "net.dial".into(),
        symbol: "otter_std_net_dial".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "net.send".into(),
        symbol: "otter_std_net_send".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "net.recv".into(),
        symbol: "otter_std_net_recv".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "net.close".into(),
        symbol: "otter_std_net_close".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "net.http_get".into(),
        symbol: "otter_std_net_http_get".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "net.http_post".into(),
        symbol: "otter_std_net_http_post".into(),
        signature: FfiSignature::new(vec![FfiType::Str, FfiType::Str], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "net.response.status".into(),
        symbol: "otter_std_net_response_status".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "net.response.body".into(),
        symbol: "otter_std_net_response_body".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_net_symbols,
    }
}
