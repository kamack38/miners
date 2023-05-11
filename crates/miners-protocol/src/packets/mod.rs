use crate::packet::IntoPacket;

pub mod handshake;
pub mod status;
pub mod login;

pub struct EmptyPacket(pub i32);

impl IntoPacket for EmptyPacket {
    fn into_packet(self, _protocol_version: i32) -> crate::packet::RawPacket {
        crate::packet::RawPacket::empty(self.0)
    }
}