use std::{
    error::Error,
    io::{stdin, stdout, BufReader, Cursor, ErrorKind, Write},
    net::UdpSocket,
    process,
    sync::mpsc,
    thread,
    time::Duration,
};

use bytelines::ByteLines;

use crate::sodium::{
    self, Nonce, PublicKey, SecretKey, NONCE_BYTES, PUBLIC_KEY_BYTES, SECRET_KEY_BYTES,
};

const MSG_SIZE: usize = 1024;

use std::convert::TryInto;

fn to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

fn handle_user_input(
    socket: UdpSocket,
    addr: &str,
    receiver: mpsc::Receiver<Vec<Vec<u8>>>,
    sk: SecretKey,
) -> Result<(), Box<dyn Error>> {
    let mut send_addr = String::from(addr);
    let dst_key;
    let dst_public_key;
    let public_key = sk.public_key().key().to_vec();

    println!("Waiting for chat mate");

    loop {
        // Sending just our public_key, so the receiver could decrypt our
        // message body
        socket.send_to(
            [public_key.as_slice(), b"\n"].concat().as_slice(),
            &send_addr,
        )?;

        // Trying to receive socket address and public key of destination
        match receiver.try_recv() {
            Ok(v) => {
                send_addr = String::from_utf8(v[0].clone())?;
                dst_key = v[1].clone();
                let a = to_array::<u8, PUBLIC_KEY_BYTES>(v[1].clone());
                dst_public_key = PublicKey::new(a);
                break;
            }
            Err(_) => (),
        }

        thread::sleep(Duration::from_millis(100));
    }

    socket.send_to(
        [public_key.as_slice(), b"\n"].concat().as_slice(),
        &send_addr,
    )?;

    println!("Type a message and hit Enter to send it");
    println!("To quit type :quit and hit Enter");

    loop {
        print!("message: ");
        stdout().flush()?;

        let msg = get_input();

        let n = sodium::gen_nonce();
        let encoded_msg = sodium::easy(msg.as_bytes(), &n, &dst_public_key, &sk);

        // Send message as a sequence of bytes separated by new line
        // First line is our public key i.e. message's source key
        // Second line is the destination public key
        // Third line is nonce value we used to encode the message with our
        // private key and the receiver's public key
        // Last line is the actual message encoded with libsodium
        let bytes_to_send = [
            public_key.as_slice(), b"\n",
            dst_key.as_slice(), b"\n",
            n.value(), b"\n",
            encoded_msg.as_slice(),
        ]
        .concat();

        socket.send_to(bytes_to_send.as_slice(), &send_addr)?;

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

fn read_from_socket(socket: UdpSocket, sender: mpsc::Sender<Vec<Vec<u8>>>, sk: SecretKey) {
    let local_addr = socket.local_addr().unwrap();
    let mut addr_sent = false;
    let pk = sk.public_key();
    loop {
        let mut buff = [0; MSG_SIZE];

        if let Ok((_, addr)) = socket.recv_from(&mut buff) {
            if local_addr != addr {
                let buff = Cursor::new(buff);

                let reader = BufReader::new(buff);

                // Read message line by line
                let mut lines = ByteLines::new(reader);

                // First line is the sender's public key
                // We will need it to decode the message
                let src_key;
                match lines.next() {
                    Some(Ok(k)) => src_key = k,
                    _ => continue,
                }

                let src_pk = PublicKey::new(to_array::<u8, PUBLIC_KEY_BYTES>(src_key.to_vec()));

                if !addr_sent {
                    // We need to send the sender's public key to the thread
                    // responsible for sending messages
                    sender
                        .send(vec![format!("{}", addr).into_bytes(), src_key.to_vec()])
                        .unwrap();
                    addr_sent = true;
                    // If we've just got the sender's public key, then the rest
                    // of the message is empty
                    continue;
                }

                // Second line is the destination public key i.e. our public key
                let dst_key;
                match lines.next() {
                    Some(Ok(k)) => dst_key = k,
                    _ => continue,
                }

                if dst_key != pk.key() {
                    continue;
                }

                // Third line is the nonce to decode body of the message
                let nonce;
                match lines.next() {
                    Some(Ok(n)) => nonce = Nonce::new(to_array::<u8, NONCE_BYTES>(n.to_vec())),
                    _ => continue,
                }

                // Final line is the actual message
                let mut msg: Vec<u8>;
                match lines.next() {
                    Some(Ok(m)) => msg = m.to_vec().into_iter().filter(|x| *x != 0).collect(),
                    _ => continue,
                }

                // Decoding the message
                match sodium::open(msg.as_ref(), &nonce, &src_pk, &sk) {
                    Ok(m) => msg = m,
                    Err(err) => println!("Err: {:#?}", err),
                }

                // Parsing message to utf-8 string
                let msg = match String::from_utf8(msg) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                // `:quit` message is a special one. If it was send to us, then
                // our chat mate has terminated the session
                if msg.trim() == ":quit" {
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

    let (sender, receiver) = mpsc::channel::<Vec<Vec<u8>>>();

    sodium::init();
    let (_, sk) = sodium::gen_keypair();

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

    let sk_copy = SecretKey::new(to_array::<u8, SECRET_KEY_BYTES>(sk.key().to_vec()));

    let handle = thread::spawn(move || read_from_socket(read_socket, sender, sk));

    handle_user_input(socket, &addr, receiver, sk_copy)?;

    handle.join().unwrap();

    Ok(())
}
