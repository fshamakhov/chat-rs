use std::{
    error::Error,
    io::{stdin, stdout, ErrorKind, Write},
    net::UdpSocket,
    process,
    sync::mpsc,
    thread,
};

const MSG_SIZE: usize = 1024;

fn handle_user_input(
    socket: UdpSocket,
    addr: &str,
    receiver: mpsc::Receiver<String>,
) -> Result<(), Box<dyn Error>> {
    println!("Type a message and hit Enter to send it");
    println!("To quit type :quit and hit Enter");

    let mut received = false;
    let mut send_addr = String::from(addr);

    loop {
        print!("message: ");
        stdout().flush()?;

        let msg = get_input();

        if !received {
            match receiver.try_recv() {
                Ok(a) => {
                    send_addr = a;
                    received = true;
                }
                Err(_) => (),
            }
        }

        socket.send_to(msg.as_bytes(), &send_addr)?;

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

fn read_from_socket(socket: UdpSocket, sender: mpsc::Sender<String>) {
    let local_addr = socket.local_addr().unwrap();
    let mut addr_sent = false;
    loop {
        let mut buff = [0; MSG_SIZE];

        if let Ok((amt, addr)) = socket.recv_from(&mut buff) {
            if local_addr != addr {
                if !addr_sent {
                    sender.send(format!("{}", addr)).unwrap();
                    addr_sent = true;
                }

                let msg: Vec<u8> = buff.into_iter().take(amt).collect();
                let msg = String::from_utf8(msg).expect("Invalid utf8 message");

                if msg == ":quit" {
                    println!("Chat session has been terminated");
                    process::exit(0);
                }

                println!();
                println!("chat: {msg}");
                print!("message: ");

                stdout().flush().expect("Stdout error");
            }
        }
    }
}

pub fn run(host: &str, port: &str) -> Result<(), Box<dyn Error>> {
    let addr = format!("{host}:{port}");
    let socket;

    let (sender, receiver) = mpsc::channel();

    match UdpSocket::bind(&addr[..]) {
        Ok(s) => socket = s,
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            socket = UdpSocket::bind(format!("{host}:0"))?;
        }
        Err(err) => {
            println!("Could not create socket: {:#?}", err);
            process::exit(1);
        }
    }

    let read_socket = socket.try_clone()?;

    let handle = thread::spawn(move || read_from_socket(read_socket, sender));

    handle_user_input(socket, &addr, receiver)?;

    handle.join().unwrap();

    Ok(())
}
