use std::{
    env,
    ffi::{c_char, c_int, c_void},
};

use ordering_server::{
    FederateId, HookId, HookInvocation, SequenceNumberByFileAndLine, ORDSERV_PORT_ENV_VAR,
};

#[no_mangle]
pub static ORDERING_CLIENT_API: OrderingClientApi = OrderingClientApi {
    start_client,
    drop_join_handle,
    tracepoint_maybe_wait,
    tracepoint_maybe_notify,
    tracepoint_maybe_do,
};

#[repr(C)]
pub struct OrderingClientApi {
    start_client: unsafe extern "C" fn(fedid: c_int) -> ClientAndJoinHandle,
    drop_join_handle: unsafe extern "C" fn(join_handle: *mut c_void),
    tracepoint_maybe_wait: unsafe extern "C" fn(
        client: *mut c_void,
        hook_id: *const c_char,
        federate_id: c_int,
        sequence_number: c_int,
    ),
    tracepoint_maybe_notify: unsafe extern "C" fn(
        client: *mut c_void,
        hook_id: *const c_char,
        federate_id: c_int,
        sequence_number: c_int,
    ),
    tracepoint_maybe_do: unsafe extern "C" fn(
        client: *mut c_void,
        hook_id: *const c_char,
        federate_id: c_int,
        sequence_number: c_int,
    ),
}

#[repr(C)]
pub struct ClientAndJoinHandle {
    client: *mut c_void,
    join_handle: *mut c_void,
}

/// # Safety
///
/// Only operate on the return value of this function by passing it as the first argument to other
/// functions defined in this library. It is not necessary to hold a mutex before operating on the
/// return value of this function; it is already protected by a mutex internally.
#[no_mangle]
pub unsafe extern "C" fn start_client(fedid: c_int) -> ClientAndJoinHandle {
    println!("Starting client");
    #[allow(clippy::unnecessary_cast)]
    let (client, join_handle) = ordering_server::client::BlockingClient::start(
        (
            "127.0.0.1",
            env::var(ORDSERV_PORT_ENV_VAR).unwrap().parse().unwrap(),
        ),
        fedid as i32,
    );
    println!("Client started");
    ClientAndJoinHandle {
        client: Box::into_raw(Box::new(client)) as *mut c_void,
        join_handle: Box::into_raw(Box::new(join_handle)) as *mut c_void,
    }
}

/// Terminate the client thread (which in Rust, is done by dropping the `JoinHandle`).
///
/// # Safety
///
/// This function invalidates is argument (by freeing it).
#[no_mangle]
pub unsafe extern "C" fn drop_join_handle(join_handle: *mut c_void) {
    let _ = Box::from_raw(join_handle as *mut std::thread::JoinHandle<()>);
}

/// # Safety
///
/// This function may block the current thread. Its argument must be the "client" field of the
/// return value of `start_client`.
#[no_mangle]
pub unsafe extern "C" fn tracepoint_maybe_wait(
    client: *mut c_void,
    hook_id: *const c_char,
    federate_id: c_int,
    sequence_number: c_int,
) {
    let client = &*(client as *mut ordering_server::client::BlockingClient);
    client.tracepoint_maybe_wait(make_hook_invocation(hook_id, federate_id, sequence_number));
}

/// # Safety
///
/// The argument of this function must be the "client" field of the return value of
/// `start_client`.
#[no_mangle]
pub unsafe extern "C" fn tracepoint_maybe_notify(
    client: *mut c_void,
    hook_id: *const c_char,
    federate_id: c_int,
    sequence_number: c_int,
) {
    let client = &*(client as *mut ordering_server::client::BlockingClient);
    client.tracepoint_maybe_notify(make_hook_invocation(hook_id, federate_id, sequence_number));
}

/// # Safety
///
/// The argument of this function must be the "client" field of the return value of
/// `start_client`.
#[no_mangle]
pub unsafe extern "C" fn tracepoint_maybe_do(
    client: *mut c_void,
    hook_id: *const c_char,
    federate_id: c_int,
    sequence_number: c_int,
) {
    tracepoint_maybe_wait(client, hook_id, federate_id, sequence_number);
    tracepoint_maybe_notify(client, hook_id, federate_id, sequence_number);
}

unsafe fn make_hook_invocation(
    hook_id: *const c_char,
    federate_id: c_int,
    sequence_number: c_int,
) -> HookInvocation {
    #[allow(clippy::unnecessary_cast)]
    HookInvocation {
        hid: HookId::new(
            std::ffi::CStr::from_ptr(hook_id)
                .to_str()
                .unwrap()
                .to_string(),
            FederateId(federate_id as i32),
        ),
        seqnum: SequenceNumberByFileAndLine(sequence_number as u32),
    }
}
