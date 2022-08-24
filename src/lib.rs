use ockam::{
    authenticated_storage::InMemoryStorage,
    identity::{Identity, TrustEveryonePolicy},
    route,
    vault::Vault,
    Address, Context, Result, Route, Routed, TcpTransport, Worker, TCP,
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

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let return_route = msg.return_route().clone();
        ctx.send(return_route, "Ok".to_string()).await?;
        println!();
        println!("chat: {}", msg.body());
        print!("message: ");
        stdout().flush().expect("Stdout error");
        Ok(())
    }
}

pub struct ServerWorker {
    started_sender: bool,
    parent_address: Address,
}

impl Default for ServerWorker {
    fn default() -> Self {
        ServerWorker {
            started_sender: false,
            parent_address: Address::random_local(),
        }
    }
}

pub struct ServerSenderWorker {
    route: Route,
    parent_address: Address,
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
                self.shutdown(ctx).await?;
                ctx.send(route![self.parent_address.clone()], ":quit".to_string())
                    .await?;
                break;
            } else {
                ctx.send(self.route.clone(), msg).await?;
                ctx.receive::<String>().await?;
            }
        }
        Ok(())
    }
}

#[ockam::worker]
impl Worker for ServerWorker {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let return_route = msg.return_route().clone();
        ctx.send(return_route, "Ok".to_string()).await?;

        if !self.started_sender {
            self.started_sender = true;
            let mut message = msg.into_local_message();
            let transport_message = message.transport_mut();

            let address = transport_message.return_route.next()?;
            ctx.start_worker(
                "server_sender_worker",
                ServerSenderWorker {
                    parent_address: self.parent_address.clone(),
                    route: route![address.clone(), "client_worker"],
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

pub async fn start_server(host: &str, port: &str, mut ctx: Context) -> Result<(), Box<dyn Error>> {
    ctx.start_worker(
        "server_worker",
        ServerWorker {
            parent_address: ctx.address(),
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

    println!("Started server on port {}", port);
    println!("Waiting for chat mate...");

    let reply = ctx.receive::<String>().await?;
    if reply == ":quit".to_string() {
        println!("Bye Bye!");
        ctx.stop_worker("server_worker").await?;
        ctx.stop().await?;
    }

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

    ctx.start_worker("client_worker", ClientWorker).await?;

    let channel = identity
        .create_secure_channel(route, TrustEveryonePolicy, &storage)
        .await?;

    println!("Successfully connected to chat at {host}:{port}");

    let route = route![channel.clone(), "server_worker"];

    ctx.send(route.clone(), "Hello Ockam!".to_string()).await?;

    ctx.receive::<String>().await?;

    println!("Type a message and hit Enter to send it");
    loop {
        print!("message: ");
        stdout().flush()?;
        let msg = get_input();

        if msg == ":quit" {
            println!("Bye Bye!");
            ctx.stop_worker("client_worker").await?;
            ctx.stop().await?;
            break;
        } else {
            ctx.send(route.clone(), msg).await?;
            ctx.receive::<String>().await?;
        }
    }

    Ok(())
}
