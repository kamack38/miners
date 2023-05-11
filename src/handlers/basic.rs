use miners_protocol::packet::RawPacket;

use crate::{define_events, client::ClientPacketHandler, events::basic::DeathEvent};

#[derive(Clone)]
pub struct KeepAliveHandler;

impl ClientPacketHandler for KeepAliveHandler {
    fn handle(&self, client: &mut crate::client::MinecraftClient, packet: &miners_protocol::packet::RawPacket) {
        if client.socket.state != miners_protocol::ConnectionState::Play {
            return;
        }

        let mut packet = packet.clone();
        packet.id = 0x12;

        log::debug!(target: "miners-client", "Keep alive packet received: {}", packet.clone().read_long());
        client.socket.send_packet(packet).ok(); // Send back with new id
    }

    fn ids(&self) -> &'static [i32] {
        &[0x20]
    }
}

define_events!(KeepAlivePacketEvent (id: i64));

#[derive(Clone)]
pub struct DeathHandler;

impl ClientPacketHandler for DeathHandler {
    fn handle(&self, client: &mut crate::client::MinecraftClient, packet: &miners_protocol::packet::RawPacket) {
        if client.socket.state != miners_protocol::ConnectionState::Play {
            return;
        }

        let packet = packet.clone();
        let packet = DeathPacket::from(packet);

        log::debug!(target: "miners-client", "Death packet received: {:?}", packet);

        client.emit(DeathEvent);
    }

    fn ids(&self) -> &'static [i32] {
        &[0x36]
    }
}

#[derive(Debug, Clone)]
pub struct DeathPacket {
    pub id: i32,
    pub killer: i32,
    pub message: String,
}

impl From<RawPacket> for DeathPacket {
    fn from(mut packet: RawPacket) -> Self {
        let id = packet.read_varint();
        let killer = packet.read_int();
        let message = packet.read_string();

        DeathPacket {
            id,
            killer,
            message,
        }
    }
}