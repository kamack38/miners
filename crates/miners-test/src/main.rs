use miners::{client::{MinecraftClient, ClientConfig, ClientMutLock, ClientLockExt}, events::basic::{SpawnEvent, DeathEvent}, plugins::basic::BasicPlugin, handlers::chat::ChatMessageEvent};

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let mut client = MinecraftClient::new(ClientConfig::default());
    client.once(|client: ClientMutLock, _: &SpawnEvent| {
        let mut client = client.wl(); // Acquire write lock
        
        // Ensure that player is alive (respawn if not)
        client.respawn();
        
        // Wait for death event
        client.on(|client: ClientMutLock, _: &DeathEvent| {
            // Respawn on death
            std::thread::sleep(std::time::Duration::from_secs(10));
            client.wl().respawn();
            println!("I died, respawning!");
        });

        // Wait for chat message event
        client.on(|_client: ClientMutLock, e: &ChatMessageEvent| {
            println!("Chat message: {:?}", e.message);
        });
    });
    client.start();
}
