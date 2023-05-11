use miners::{client::{MinecraftClient, ClientConfig}, events::basic::{SpawnEvent, DeathEvent}, plugins::basic::BasicPlugin};

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let mut client = MinecraftClient::new(ClientConfig::default());
    client.once(|client: &mut MinecraftClient, _: &SpawnEvent| {
        client.respawn();
        client.on(|client: &mut MinecraftClient, _: &DeathEvent| {
            client.respawn();
            println!("I died, respawning!");
        });
    });
    client.start();
}
