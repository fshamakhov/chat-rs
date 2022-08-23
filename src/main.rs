use std::{process};

use chat::{self, Mode};
use clap::{App, Arg};
use ockam::Context;


#[ockam::node]
async fn main(ctx: Context) {
    let matches = App::new("P2P Chat")
        .version("0.1.0")
        .about("Simple p2p chat written in Rust")
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_parser(clap::builder::EnumValueParser::<Mode>::new())
                .default_value("server"),
        )
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .default_value("127.0.0.1")
                .help("Host to connect to in client mode or address to bind to in server mode"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .default_value("6000")
                .help("Host to connect to in client mode or address to bind to in server mode"),
        )
        .get_matches();

    let mode: &Mode = matches.get_one("mode").unwrap();
    let host: &String = matches.get_one("host").unwrap();
    let port: &String = matches.get_one("port").unwrap();

    if let Err(e) = match mode {
        Mode::Server => chat::start_server(host, port, ctx).await,
        Mode::Client => chat::connect_to_server(host, port, ctx).await,
    } {
        eprintln!("Application error: {e}");
        process::exit(1);

    }
}
