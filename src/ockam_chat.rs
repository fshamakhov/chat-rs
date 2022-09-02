use ockam::{
    authenticated_storage::InMemoryStorage,
    identity::{Identity, TrustEveryonePolicy},
    route,
    vault::Vault,
    Context, Result, Routed, TcpTransport, Worker, TCP,
};
use std::{
    error::Error,
    io::{stdin, stdout, Write},
    time::Duration,
};

fn get_input() -> String {
    let mut buff = String::new();

    stdin()
        .read_line(&mut buff)
        .expect("Failed to read from stdin");

    buff.trim().to_string()
}
struct ChatWorker;

#[ockam::worker]
impl Worker for ChatWorker {
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

pub async fn run(
    host: &str,
    port: &str,
    port_connect: &str,
    mut ctx: Context,
) -> Result<(), Box<dyn Error>> {
    ctx.start_worker("worker", ChatWorker).await?;

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.listen(format!("{host}:{port}")).await?;

    let vault = Vault::create();

    let identity = Identity::create(&ctx, &vault).await?;

    let storage = InMemoryStorage::new();

    identity
        .create_secure_channel_listener("chat", TrustEveryonePolicy, &storage)
        .await?;

    println!("Waiting for chat mate");

    'outer: loop {
        if let Ok(channel) = identity
            .create_secure_channel_extended(
                route![(TCP, &format!("{host}:{port_connect}")), "chat"],
                TrustEveryonePolicy,
                &storage,
                Duration::from_secs(1),
            )
            .await
        {
            println!("Type a message and hit Enter to send it");
            println!("To quit type :quit and hit Enter");
            loop {
                print!("message: ");
                stdout().flush()?;
                let msg = get_input();

                if msg == ":quit" {
                    println!("Bye Bye!");
                    ctx.stop_worker("worker").await?;
                    ctx.stop().await?;
                    break 'outer;
                } else {
                    ctx.send_and_receive(route![channel.clone(), "worker"], msg)
                        .await?;
                }
            }
        }
    }

    Ok(())
}
