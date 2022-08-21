use std::{
    env,
    error::Error,
    io::{stdin, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    process, thread,
    time::Duration,
};

const MSG_SIZE: usize = 256;

pub enum Mode {
    Server,
    Client { host: String, port: String },
}

pub fn mode(mut args: impl Iterator<Item = String>) -> Result<Mode, &'static str> {
    args.next();

    let host = match args.next() {
        Some(arg) => arg,
        None => return Ok(Mode::Server),
    };

    let port = match args.next() {
        Some(arg) => arg,
        None => return Err("Provide port to connect to"),
    };

    Ok(Mode::Client { host, port })
}

pub fn run(mode: Mode) -> Result<(), Box<dyn Error>> {
    match mode {
        Mode::Server => start_server(),
        Mode::Client { host, port } => connect_to_server(&host, &port),
    }
}

fn handle_user_input(socket: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    println!("Type a message and hit Enter to send it");

    loop {
        let msg = get_input();

        if msg == ":quit" {
            println!("Bye Bye!");
            break;
        }

        let mut buff = msg.clone().into_bytes();
        buff.resize(MSG_SIZE, 0);

        socket.write_all(&buff)?;
    }

    Ok(())
}

fn get_input() -> String {
    let mut buff = String::new();

    stdin()
        .read_line(&mut buff)
        .expect("Failed to read from stdin");

    buff.trim().to_string()
}

fn read_from_socket(socket: &mut TcpStream) {
    loop {
        let mut buff = vec![0; MSG_SIZE];

        match socket.read_exact(&mut buff) {
            Ok(_) => {
                let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                let msg = String::from_utf8(msg).expect("Invalid utf8 message");
                println!("chat: {}", msg);
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("Chat session has been terminated");
                process::exit(0);
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn start_server() -> Result<(), Box<dyn Error>> {
    let port = env::var("CHAT_PORT").unwrap_or_else(|_| String::from("6000"));
    let host = env::var("CHAT_HOST").unwrap_or_else(|_| String::from("0.0.0.0"));

    let server = TcpListener::bind(format!("{host}:{port}"))?;

    println!("Started server on port {}", port);
    println!("Waiting for chat mate...");

    if let Ok((mut socket, addr)) = server.accept() {
        println!("Client {addr} connected");

        let mut client = socket.try_clone()?;

        thread::spawn(move || read_from_socket(&mut socket));

        handle_user_input(&mut client)?
    }

    Ok(())
}

fn connect_to_server(host: &str, port: &str) -> Result<(), Box<dyn Error>> {
    let mut socket = TcpStream::connect(format!("{host}:{port}"))?;
    println!("Successfully connected to chat at {host}:{port}");

    socket.set_nonblocking(true)?;

    let mut client = socket.try_clone()?;

    thread::spawn(move || read_from_socket(&mut socket));

    handle_user_input(&mut client)?;

    Ok(())
}
