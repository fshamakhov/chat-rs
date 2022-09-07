use std::{
    error::Error,
    io::{stdin, stdout, BufReader, Cursor, ErrorKind, Write},
    net::UdpSocket,
    process,
    sync::{mpsc},
    thread,
};

use bytelines::ByteLines;

use crate::sodium::{self, PublicKey, SecretKey, PUBLIC_KEY_BYTES};

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
    println!("Type a message and hit Enter to send it");
    println!("To quit type :quit and hit Enter");

    let mut addr_received = false;
    let mut send_addr = String::from(addr);
    let mut dst_key: Vec<u8> = vec![];
    let mut dst_public_key = None;
    let public_key = sk.public_key().key().to_vec();

    loop {
        print!("message: ");
        stdout().flush()?;

        let msg = get_input();

        if !addr_received {
            match receiver.try_recv() {
                Ok(v) => {
                    send_addr = String::from_utf8(v[0].clone())?;
                    dst_key = v[1].clone();
                    let a = to_array::<u8, PUBLIC_KEY_BYTES>(v[1].clone());
                    dst_public_key = Some(PublicKey::new(a));
                    addr_received = true;
                }
                Err(_) => (),
            }
        }

        let send_msg = if addr_received {
            let n = sodium::gen_nonce();
            sodium::easy(
                msg.as_bytes(),
                &n,
                &dst_public_key.as_ref().unwrap(),
                &sk,
            )
        } else {
            vec![]
        };

        let bs = [
            public_key.as_slice(),
            "\n".as_bytes(),
            dst_key.as_slice(),
            "\n".as_bytes(),
            send_msg.as_slice(),
        ]
        .concat();

        socket.send_to(bs.as_slice(), &send_addr)?;

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

fn read_from_socket(
    socket: UdpSocket,
    sender: mpsc::Sender<Vec<Vec<u8>>>,
    pk: PublicKey,
) {
    let local_addr = socket.local_addr().unwrap();
    let mut addr_sent = false;
    let public_key = pk.key();
    loop {
        let mut buff = [0; MSG_SIZE];

        if let Ok((_, addr)) = socket.recv_from(&mut buff) {
            if local_addr != addr {
                let buff = Cursor::new(buff);

                let reader = BufReader::new(buff);

                let mut lines = ByteLines::new(reader);

                let src_key;
                let dst_key;

                match lines.next() {
                    Some(Ok(k)) => src_key = k,
                    _ => continue,
                }

                if !addr_sent {
                    sender
                        .send(vec![format!("{}", addr).into_bytes(), src_key.to_vec()])
                        .unwrap();
                    addr_sent = true;
                }

                match lines.next() {
                    Some(Ok(k)) => dst_key = k,
                    _ => continue,
                }

                if dst_key != public_key {
                    continue;
                }

                println!("dst_key: {:?}", dst_key);

                let msg;

                match lines.next() {
                    Some(Ok(m)) => msg = m.to_vec().into_iter().filter(|x| *x != 0).collect(),
                    _ => continue,
                }

                let msg = String::from_utf8(msg).expect("Invalid utf-8 message");

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
    let (pk, sk) = sodium::gen_keypair();

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

    let handle = thread::spawn(move || read_from_socket(read_socket, sender, pk));

    handle_user_input(socket, &addr, receiver, sk)?;

    handle.join().unwrap();

    Ok(())
}
