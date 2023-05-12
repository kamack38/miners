use crate::{
    packet::{IntoPacket, RawPacket},
    utils::nbt::NBTType,
};

#[derive(Debug, Clone)]
pub struct Slot {
    pub present: bool,
    pub item_id: Option<i32>,
    pub item_count: Option<u8>,
    pub nbt: Option<NBTType>,
}

impl Slot {
    pub fn from_packet(packet: &mut RawPacket) -> Result<Slot, String> {
        let present = packet.read_bool();
        Ok(Slot {
            present,
            item_id: match present {
                true => Some(packet.read_int()),
                false => None,
            },
            item_count: match present {
                true => Some(packet.read_byte()),
                false => None,
            },
            nbt: match present {
                true => Some(match NBTType.from_packet(packet) {
                    Ok(nbt) => nbt,
                    Err(e) => return e,
                }),
                false => None,
            },
        })
    }
}

impl IntoPacket for Slot {}
