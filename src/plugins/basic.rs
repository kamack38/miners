use std::time;

use miners_protocol::packet::{IntoPacket, RawPacket};

use crate::client::MinecraftClient;

/// Basic plugin trait that provides some useful methods for the client
pub trait BasicPlugin {
    /// Sends respawn packet to the server requesting respawn
    fn respawn(&mut self);
    /// Sends chat message to the server with the specified message
    /// 
    /// **Warning:** This is only temporary and experimental implementation
    fn send_chat_message(&mut self, message: String);
}

impl BasicPlugin for MinecraftClient {
    fn respawn(&mut self) {
        // Send respawn packet
        self.socket.send_packet(ClientCommandAction::PerformRespawn).ok();
    }

    fn send_chat_message(&mut self, message: String) {
        // Send chat message packet
        self.socket.send_packet(ChatMessagePacket {
            message: message.clone(),
            timestamp: time::UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
            // TODO: Implement this properly
        }).ok();
    }
}

/// Client command action packet (0x07)
/// 
/// Represents two basic actions that can be performed by the client:
/// - Respawn = 0
/// - Request stats = 1
pub enum ClientCommandAction {
    PerformRespawn = 0,
    RequestStats = 1,
}

impl IntoPacket for ClientCommandAction {
    fn into_packet(self, _protocol_version: i32) -> RawPacket {
        let mut packet = RawPacket::empty(0x07);
        packet.write_varint(self as i32);
        packet
    }
}

/// Chat message packet (0x05)
/// 
/// **Warning:** This is only temporary and experimental implementation
pub struct ChatMessagePacket {
    pub message: String,
    pub timestamp: u64,
}

impl IntoPacket for ChatMessagePacket {
    fn into_packet(self, _protocol_version: i32) -> RawPacket {
        let mut packet = RawPacket::empty(0x05);

        packet.write_string(&self.message);
        packet.write_ulong(self.timestamp);

        packet.write_long(0); // Salt
        packet.write_varint(0); // No signature

        packet.write_bool(false); // No signed preview
        packet.write_varint(0); // No previous messages

        packet.write_bool(false); // Has last message

        packet
    }
}

