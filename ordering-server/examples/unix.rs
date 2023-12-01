use std::{
    os::{fd::IntoRawFd, unix},
    process::Command,
};

extern crate libc;

fn main() {
    // let (a, b) = unix::net::UnixStream::pair().unwrap();
    // let a = a.into_raw_fd();
    // unsafe {
    //     let flags = libc::fcntl(a, libc::F_GETFD);
    //     libc::fcntl(a, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
    // }
    // let b = b.into_raw_fd();
    // println!("DEBUG: a: {:?}", a);
    // println!("DEBUG: b: {:?}", b);
    // tokio::runtime::Builder::new_current_thread()
    //     .enable_all()
    //     .build()
    //     .unwrap()
    //     .block_on(async {
    //         // let a = unsafe { ordering_server::tcpconnectionprovider::socket_from_raw_fd(a) };
    //         // println!("a: {:?}", a);
    //         let b = unsafe { ordering_server::tcpconnectionprovider::socket_from_raw_fd(b) };
    //         println!("b: {:?}", b);
    //     });
    // Command::new("cargo")
    //     .args(["run", "--example", "unix2"])
    //     .env("FD", a.to_string())
    //     .spawn()
    //     .unwrap()
    //     .wait()
    //     .unwrap();
}
