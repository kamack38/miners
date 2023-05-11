use miners_protocol::packet::{IntoPacket, RawPacket};

use crate::client::MinecraftClient;

pub trait BasicPlugin {
    fn respawn(&mut self);
}

impl BasicPlugin for MinecraftClient {
    fn respawn(&mut self) {
        self.socket.send_packet(ClientCommandAction::PerformRespawn).ok();
    }
}

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