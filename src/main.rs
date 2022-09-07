use std::process;

use clap::{App, Arg};

pub mod ockam_chat;
pub mod tcp_chat;
pub mod udp_chat;
pub mod sodium;

fn main() {
    let matches = App::new("P2P Chat")
        .version("0.1.0")
        .about("Simple p2p chat written in Rust with encrypted channel")
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .default_value("127.0.0.1")
                .help("IP address to connect or listen to"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .default_value("6000")
                .help("Port to connect or listen to"),
        )
        .get_matches();

    let host: &String = matches.get_one("host").unwrap();
    let port: &String = matches.get_one("port").unwrap();

    if let Err(e) = udp_chat::run(host, port) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
