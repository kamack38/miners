use crate::packet::IntoPacket;

#[derive(Debug, Clone)]
pub struct HandshakePacket {
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

impl HandshakePacket {
    /// Creates a new handshake packet for pinging the server
    pub fn new_ping(server_address: String, server_port: u16) -> HandshakePacket {
        HandshakePacket {
            server_address,
            server_port,
            next_state: 1,
        }
    }

    /// Creates a new handshake packet for logging into the server
    pub fn new_login(server_address: String, server_port: u16) -> HandshakePacket {
        HandshakePacket {
            server_address,
            server_port,
            next_state: 2,
        }
    }
}

impl IntoPacket for HandshakePacket {
    fn into_packet(self, protocol_version: i32) -> crate::packet::RawPacket {
        let mut packet = crate::packet::RawPacket::empty(0);
        packet.write_varint(protocol_version);
        packet.write_string(&self.server_address);
        packet.write_ushort(self.server_port);
        packet.write_varint(self.next_state);
        packet
    }
}