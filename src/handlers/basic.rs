use miners_protocol::packet::RawPacket;

use crate::{define_events, client::{ClientPacketHandler, ClientMutLock, ClientLockExt}, events::basic::DeathEvent};

/// Handles keep alive packets (0x20) which are sent by the server and must be responded to with the same data
#[derive(Clone)]
pub struct KeepAliveHandler;

impl ClientPacketHandler for KeepAliveHandler {
    fn handle(&self, client: ClientMutLock, packet: &miners_protocol::packet::RawPacket) {
        // Ensure that we are in play state
        if client.get_state() != miners_protocol::ConnectionState::Play {
            return;
        }

        // Clone packet to change its id
        let mut packet = packet.clone();
        packet.id = 0x12;

        log::debug!(target: "miners-client", "Keep alive packet received: {}", packet.clone().read_long());
        // Send same data back to the server with new id (0x12)
        client.write().unwrap().socket.send_packet(packet).ok();
    }

    fn ids(&self) -> &'static [i32] {
        &[0x20]
    }
}

define_events!(KeepAlivePacketEvent (id: i64) => "Emitted when keep alive packet is received");

/// Handles death packets (0x36) which are sent by the server when player dies
#[derive(Clone)]
pub struct DeathHandler;

impl ClientPacketHandler for DeathHandler {
    fn handle(&self, client: ClientMutLock, packet: &miners_protocol::packet::RawPacket) {
        // Ensure that we are in play state
        if client.get_state() != miners_protocol::ConnectionState::Play {
            return;
        }

        // Clone packet to read it
        let packet = packet.clone();
        // Convert packet to DeathPacket struct
        let packet = DeathPacket::from(packet);

        log::debug!(target: "miners-client", "Death packet received: {:?}", packet);

        // Emit event with appropriate data
        client.emit(DeathEvent);
    }

    fn ids(&self) -> &'static [i32] {
        &[0x36]
    }
}

/// Represents death packet (0x36) sent by the server when player dies
/// 
/// Contains player id, killer id and death message
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