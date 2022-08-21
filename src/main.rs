use std::{env, process};

use chat;

fn main() {
    let mode = chat::mode(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    if let Err(e) = chat::run(mode) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
