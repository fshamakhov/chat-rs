use std::{
    error::Error,
    io::{stdin, stdout, ErrorKind, Write, Read},
    net::{TcpListener, TcpStream},
    thread, time::Duration, process,
};

const MSG_SIZE: usize = 1024;

fn handle_user_input(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    println!("Type a message and hit Enter to send it");
    println!("To quit type :quit and hit Enter");

    // Terminate message with new line
    //
    // loop {
    //     print!("message: ");
    //     stdout().flush()?;
    //     let mut msg = get_input();

    //     if msg == ":quit" {
    //         println!("Bye Bye!");
    //         break;
    //     }

    //     msg = format!("{msg}\n\r");
    //     socket.write_all(msg.as_bytes())?;
    // }


    loop {
        print!("message: ");
        stdout().flush()?;
        let msg = get_input();

        if msg == ":quit" {
            println!("Bye Bye!");
            process::exit(0);
        }

        let mut buff = msg.clone().into_bytes();
        buff.resize(MSG_SIZE, 0);

        socket.write_all(&buff)?;
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
    // Try to use BufReader and read line by line
    //
    // let buf_reader = BufReader::new(stream);
    // for line in buf_reader.lines() {
    //     match line {
    //         Ok(msg) => {
    //             println!("chat: {msg}");
    //             print!("message: ");
    //             stdout().flush().expect("Stdout error");
    //         }
    //         Err(err) if err.kind() == ErrorKind::WouldBlock => (),
    //         Err(_) => {
    //             println!("Chat session has been terminated");
    //             break;
    //         }
    //     }
    // }

    loop {
        let mut buff = [0; MSG_SIZE];

        match stream.read_exact(&mut buff) {
            Ok(_) => {
                let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                let msg = String::from_utf8(msg).expect("Invalid utf8 message");
                println!();
                println!("chat: {}", msg);
                print!("message: ");
                stdout().flush().expect("Stdout error");
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
