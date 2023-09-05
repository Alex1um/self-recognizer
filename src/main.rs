use core::panic;
use std::{
    collections::HashMap,
    env,
    net::*,
    time::{Duration, Instant},
};

const TIMEOUT: u64 = 3;

fn check_hashmap(copies: &mut HashMap<SocketAddr, Instant>) {
    let prev_len = copies.len();
    copies.retain(|k, v| {
        let is_ok = v.elapsed().as_secs() < TIMEOUT;
        if !is_ok {
            println!("copy {} timed out", k);
        }
        is_ok
    });
    if copies.len() < prev_len {
        println!("current copies: {:?}", copies.keys());
    }
}

fn main() {
    let mut args = env::args();
    let ip: IpAddr = args
        .nth(1)
        .or_else(|| {
            println!("Multicast group address argument not found. Using 224.2.2.4");
            Some(String::from("224.2.2.4"))
        })
        .expect("argument")
        .parse()
        .expect("valid string at arg2");
    if !ip.is_multicast() {
        panic!("Multicast address required");
    }
    let socket = match ip {
        IpAddr::V4(ref ip) => {
            let socket: UdpSocket = UdpSocket::bind("0.0.0.0:48666").expect("Failed to bind ipv4");
            socket
                .join_multicast_v4(ip, &Ipv4Addr::new(192, 168, 51, 0))
                .expect("valid join IPv4 multicast group");
            // socket
            //     .set_multicast_loop_v4(false)
            //     .expect("setted loop option");
            socket
        }
        IpAddr::V6(ref ip) => {
            let socket = UdpSocket::bind("[::]:48666").expect("Failed to bind ipv6");
            socket
                .join_multicast_v6(ip, 0)
                .expect("valid join IPv6 multicast group");
            socket
                .set_multicast_loop_v6(false)
                .expect("setted loop option");
            socket
        }
    };

    // socket.set_ttl(2).expect("setted ttl");
    socket
        .set_read_timeout(Some(Duration::from_secs(TIMEOUT)))
        .expect("setted timeout");

    let send_socket = socket.try_clone().expect("socket clone");

    let send_handle = std::thread::spawn(move || {
        let ip = SocketAddr::new(ip, 48666);
        loop {
            send_socket.send_to("".as_bytes(), ip).expect("valid send");
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    let mut buffer = [0u8; 64];
    let mut copies = HashMap::<SocketAddr, Instant>::new();

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((_, addr)) => {
                match copies.entry(addr) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        println!("New copy found: {:?}", addr);
                        e.insert(Instant::now());
                        println!("current copies: {:?}", copies.keys());
                    }
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        e.insert(Instant::now());
                    }
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {}
                _ => {
                    println!("recv error: {}", e);
                    break;
                }
            },
        }
        check_hashmap(&mut copies)
    }

    send_handle.join().expect("successful join");
}
