use ockam::{
    authenticated_storage::InMemoryStorage,
    identity::{Identity, TrustEveryonePolicy},
    route,
    vault::Vault,
    Context, Result, Route, Routed, TcpTransport, Worker, TCP,
};
use std::{
    error::Error,
    io::{stdin, stdout, Write},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Server,
    Client,
}

impl clap::ValueEnum for Mode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Client, Self::Server]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::PossibleValue<'a>> {
        match self {
            Self::Client => Some(clap::PossibleValue::new("client")),
            Self::Server => Some(clap::PossibleValue::new("server")),
        }
    }
}

pub struct ClientWorker;

#[ockam::worker]
impl Worker for ClientWorker {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, _: &mut Context, msg: Routed<String>) -> Result<()> {
        println!();
        println!("chat: {}", msg.body());
        print!("message: ");
        stdout().flush().expect("Stdout error");
        Ok(())
    }
}

pub struct ServerWorker {
    started_sender: bool,
}

impl Default for ServerWorker {
    fn default() -> Self {
        ServerWorker {
            started_sender: false,
        }
    }
}

pub struct ServerSenderWorker {
    route: Route,
}

#[ockam::worker]
impl Worker for ServerSenderWorker {
    type Context = Context;
    type Message = String;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Type a message and hit Enter to send it");
        loop {
            print!("message: ");
            stdout().flush().expect("Stdout error");
            let msg = get_input();

            if msg == ":quit" {
                println!("Bye Bye!");
                return ctx.stop().await;
            } else {
                ctx.send(self.route.clone(), msg).await?;
            }
        }
    }
}

#[ockam::worker]
impl Worker for ServerWorker {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        if !self.started_sender {
            self.started_sender = true;
            let mut message = msg.into_local_message();
            let transport_message = message.transport_mut();

            let address = transport_message.return_route.next()?;
            ctx.start_worker(
                "sender",
                ServerSenderWorker {
                    route: route![address.clone(), "worker"],
                },
            )
            .await?;
        } else {
            println!();
            println!("chat: {}", msg.body());
            print!("message: ");
            stdout().flush().expect("Stdout error");
        }
        Ok(())
    }
}

fn get_input() -> String {
    let mut buff = String::new();

    stdin()
        .read_line(&mut buff)
        .expect("Failed to read from stdin");

    buff.trim().to_string()
}

pub async fn start_server(host: &str, port: &str, ctx: Context) -> Result<(), Box<dyn Error>> {
    ctx.start_worker(
        "foobar",
        ServerWorker {
            ..Default::default()
        },
    )
    .await?;

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.listen(format!("{host}:{port}")).await?;

    let vault = Vault::create();

    let identity = Identity::create(&ctx, &vault).await?;

    let storage = InMemoryStorage::new();

    identity
        .create_secure_channel_listener("server", TrustEveryonePolicy, &storage)
        .await?;

    Ok(())
}

pub async fn connect_to_server(
    host: &str,
    port: &str,
    mut ctx: Context,
) -> Result<(), Box<dyn Error>> {
    TcpTransport::create(&ctx).await?;

    let vault = Vault::create();

    let identity = Identity::create(&ctx, &vault).await?;

    let storage = InMemoryStorage::new();

    let route = route![(TCP, format!("{host}:{port}")), "server"];

    ctx.start_worker("worker", ClientWorker).await?;

    let channel = identity
        .create_secure_channel(route, TrustEveryonePolicy, &storage)
        .await?;

    let route = route![channel, "foobar"];

    ctx.send(route.clone(), "Hello Ockam!".to_string()).await?;

    println!("Type a message and hit Enter to send it");
    loop {
        print!("message: ");
        stdout().flush()?;
        let msg = get_input();

        if msg == ":quit" {
            println!("Bye Bye!");
            ctx.stop().await?;
            break;
        } else {
            ctx.send(route.clone(), msg).await?;
        }
    }

    Ok(())
}
