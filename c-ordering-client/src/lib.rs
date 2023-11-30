use std::{
    env,
    ffi::{c_char, c_int, c_void},
    time::Duration,
};

use ordering_server::{
    client::{BlockingClient, BlockingClientJoinHandle},
    connection::{ReadConnection, WriteConnection, UNIX_CONNECTION_MANAGEMENT},
    FederateId, HookId, HookInvocation, SequenceNumberByFileAndLine,
    ORDSERV_WAIT_TIMEOUT_MILLISECONDS_ENV_VAR,
};

use log::{debug, info};

#[no_mangle]
pub static ORDERING_CLIENT_API: OrderingClientApi = OrderingClientApi {
    start_client,
    finish,
    tracepoint_maybe_wait,
    tracepoint_maybe_notify,
    tracepoint_maybe_do,
};

#[repr(C)]
pub struct OrderingClientApi {
    start_client: unsafe extern "C" fn(fedid: c_int) -> ClientAndJoinHandle,
    finish: unsafe extern "C" fn(client_and_join_handle: ClientAndJoinHandle),
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
    simple_logger::init_with_level(log::Level::Warn).unwrap();
    info!("Starting client");
    #[allow(clippy::unnecessary_cast)]
    let (client, join_handle) = ordering_server::client::BlockingClient::start_reusing_connection(
        FederateId(fedid as i32),
        Duration::from_millis(
            env::var(ORDSERV_WAIT_TIMEOUT_MILLISECONDS_ENV_VAR)
                .unwrap()
                .parse()
                .unwrap(),
        ),
    );
    info!("Client started");
    ClientAndJoinHandle {
        client: Box::into_raw(Box::new(client)) as *mut c_void,
        join_handle: Box::into_raw(Box::new(join_handle)) as *mut c_void,
    }
}

/// Wait for the client thread to finish its final tasks, and GC its resources.
///
/// # Safety
///
/// This function invalidates its argument (by freeing it).
#[no_mangle]
pub unsafe extern "C" fn finish(client_and_join_handle: ClientAndJoinHandle) {
    info!("Shutting down client");
    let client = Box::from_raw(client_and_join_handle.client as *mut BlockingClient);
    info!("Sending halt message");
    client.halt.send(()).unwrap();
    info!("Recovering join handle");
    let join_handle =
        Box::from_raw(client_and_join_handle.join_handle as *mut BlockingClientJoinHandle);
    debug!("Joining client thread");
    drop(client);
    let (inner_client, read) = join_handle.join().unwrap();
    debug!("Client thread joined");
    (UNIX_CONNECTION_MANAGEMENT.unborrow)((
        ReadConnection::new(read),
        WriteConnection {
            stream: inner_client.connection.stream,
        },
    ));
    debug!("Exiting.");
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
    let client = &*(client as *mut BlockingClient);
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
    let client = &*(client as *mut BlockingClient);
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
