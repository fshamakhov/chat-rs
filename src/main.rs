use std::process;

use chat;
use clap::{App, Arg};
use ockam::Context;

#[ockam::node]
async fn main(ctx: Context) {
    let matches = App::new("P2P Chat")
        .version("0.1.0")
        .about("Simple p2p chat written in Rust with encrypted channel")
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .default_value("127.0.0.1")
                .help("IP address to connect to"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .default_value("6000")
                .help("Port to listen to"),
        )
        .arg(
            Arg::new("port_connect")
                .short('c')
                .long("port_connect")
                .default_value("6000")
                .help("Port to connect to"),
        )
        .get_matches();

    let host: &String = matches.get_one("host").unwrap();
    let port: &String = matches.get_one("port").unwrap();
    let port_connect: &String = matches.get_one("port_connect").unwrap();

    if let Err(e) = chat::run(host, port, port_connect, ctx).await {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
