use miners::{client::{MinecraftClient, ClientConfig, ClientMutLock, ClientLockExt}, events::basic::{SpawnEvent, DeathEvent}, plugins::basic::BasicPlugin, handlers::chat::{ChatMessageEvent, ChatMessageSource}};

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
        client.on(|client: ClientMutLock, e: &ChatMessageEvent| {
            // Ignore messages sent by this client
            if e.message.source == (&client).into() {
                return;
            }
            
            // Print message
            let mut client = client.wl();
            client.send_chat_message(format!("You said: {}", e.message.plain_message));

            // Disconnect if message is "leave"
            if e.message.plain_message == "leave" {
                client.disconnect();
            }
        });
    });
    client.start();
}
