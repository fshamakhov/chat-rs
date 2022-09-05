use std::{
    error::Error,
    io::{stdin, stdout, Write, BufReader, BufRead},
    net::{TcpListener, TcpStream},
    thread, process,
};

fn handle_user_input(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    println!("Type a message and hit Enter to send it");
    println!("To quit type :quit and hit Enter");

    loop {
        print!("message: ");
        stdout().flush()?;
        let msg = get_input();

        socket.write_all(format!("{msg}\n").as_bytes())?;

        if msg == ":quit" {
            println!("Bye Bye!");
            process::exit(0);
        }
    }
}

fn get_input() -> String {
    let mut buff = String::new();

    stdin()
        .read_line(&mut buff)
        .expect("Failed to read from stdin");

    buff.trim().to_string()
}

fn read_from_stream(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    for line in buf_reader.lines() {
        match line {
            Ok(msg) => {
                if msg == ":quit" {
                    println!("Chat session has been terminated");
                    process::exit(0);
                }
                println!();
                println!("chat: {msg}");
                print!("message: ");
                stdout().flush().expect("Stdout error");
            }
            Err(_) => {
                println!("Chat session has been terminated");
                process::exit(0);
            }
        }
    }
}

fn handle_stream(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let read_stream = stream.try_clone()?;
    let handle = thread::spawn(move || read_from_stream(read_stream));

    handle_user_input(stream)?;

    handle.join().unwrap();
    Ok(())
}

pub fn run(host: &str, port: &str) -> Result<(), Box<dyn Error>> {
    if let Ok(stream) = TcpStream::connect(format!("{host}:{port}")) {
        println!("Connected to {host}:{port}");
        handle_stream(stream)?;
    }

    let listener = TcpListener::bind(format!("{host}:{port}"))?;

    if let Ok((stream, addr)) = listener.accept() {
        println!("Accepted from {addr}");
        handle_stream(stream)?;
    }

    Ok(())
}
