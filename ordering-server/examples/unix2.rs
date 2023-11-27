use std::env;

fn main() {
    let fd = env::var("FD").unwrap().parse().unwrap();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            println!("DEBUG: fd: {:?}", fd);
            let a = unsafe { ordering_server::tcpconnectionprovider::socket_from_raw_fd(fd) };
            println!("a: {:?}", a);
        });
}
