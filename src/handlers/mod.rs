use crate::client::MinecraftClient;

pub mod basic;
pub mod chat;

pub fn register_all_handlers(client: &mut MinecraftClient) {
    client.register_packet_handler(basic::KeepAliveHandler);
    client.register_packet_handler(basic::DeathHandler);
    
    client.register_packet_handler(chat::ChatHandler);
}