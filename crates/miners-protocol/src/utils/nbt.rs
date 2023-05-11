use std::{fmt::Debug, collections::HashMap};

use crate::packet::RawPacket;

#[derive(Clone, Debug)]
pub struct NBTCompound {
    pub data: HashMap<String, NBTType>
}

#[derive(Clone)]
pub enum NBTType {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NBTType>),
    Compound(NBTCompound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Debug for NBTType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.into_snbt())
    }
}

impl NBTCompound {
    pub fn new() -> NBTCompound {
        NBTCompound {
            data: HashMap::new()
        }
    }
}

impl NBTType {
    pub fn from_packet(packet: &mut RawPacket) -> Result<NBTType, String> {
        let typeid = packet.read_byte();
        let _name = packet.read_string_ushort();
        NBTType::from_packet_raw(packet, typeid)
    }

    /// Converts the NBT type into String NBT (SNBT)
    pub fn into_snbt(&self) -> String {
        match self {
            NBTType::End => String::from(""),
            NBTType::Byte(value) => format!("{}b", value),
            NBTType::Short(value) => format!("{}s", value),
            NBTType::Int(value) => format!("{}i", value),
            NBTType::Long(value) => format!("{}l", value),
            NBTType::Float(value) => format!("{}f", value),
            NBTType::Double(value) => format!("{}d", value),
            NBTType::ByteArray(value) => format!("[B;{}]", value.iter().map(|v| format!("{}b", v)).collect::<Vec<String>>().join(",")),
            NBTType::String(value) => format!("\"{}\"", value),
            NBTType::List(value) => format!("[{}]", value.iter().map(|v| v.into_snbt()).collect::<Vec<String>>().join(",")),
            NBTType::Compound(value) => format!("{{{}}}", value.data.iter().map(|(k, v)| format!("{}:{}", k, v.into_snbt())).collect::<Vec<String>>().join(",")),
            NBTType::IntArray(value) => format!("[I;{}]", value.iter().map(|v| format!("{}i", v)).collect::<Vec<String>>().join(",")),
            NBTType::LongArray(value) => format!("[L;{}]", value.iter().map(|v| format!("{}l", v)).collect::<Vec<String>>().join(",")),
        }
    }

    pub fn from_packet_raw(packet: &mut RawPacket, typeid: u8) -> Result<NBTType, String> {
        match typeid {
            0x00 => Ok(NBTType::End), // End
            0x01 => Ok(NBTType::Byte(packet.read_byte() as i8)), // signed byte
            0x02 => Ok(NBTType::Short(packet.read_short())), // short
            0x03 => Ok(NBTType::Int(packet.read_int())), // int
            0x04 => Ok(NBTType::Long(packet.read_long())), // long
            0x05 => Ok(NBTType::Float(packet.read_float())), // float
            0x06 => Ok(NBTType::Double(packet.read_double())), // double
            0x07 => { // byte array
                let mut array = Vec::new();
                let length = packet.read_int();
                for _ in 0..length {
                    array.push(packet.read_byte() as i8);
                }
                Ok(NBTType::ByteArray(array))
            },
            0x08 => Ok(NBTType::String(packet.read_string_ushort())), // string
            0x09 => { // List
                let item_typeid = packet.read_byte();
                let length = packet.read_int();
                let mut list = Vec::new();
                for _ in 0..length {
                    list.push(NBTType::from_packet_raw(packet, item_typeid)?);
                }
                Ok(NBTType::List(list))
            },
            0x0A => { // NBTCompound
                let mut compound = NBTCompound::new();
                loop {
                    let typeid = packet.read_byte();
                    match typeid {
                        0x00 => break, // End
                        _ => {
                            let name = packet.read_string_ushort();
                            compound.data.insert(name, NBTType::from_packet_raw(packet, typeid)?);
                        }
                    }
                }
                Ok(NBTType::Compound(compound))
            },
            0x0B => { // int array
                let mut array = Vec::new();
                let length = packet.read_int();
                for _ in 0..length {
                    array.push(packet.read_int());
                }
                Ok(NBTType::IntArray(array))
            },
            0x0C => { // long array
                let mut array = Vec::new();
                let length = packet.read_int();
                for _ in 0..length {
                    array.push(packet.read_long());
                }
                Ok(NBTType::LongArray(array))
            },
            _ => Err(format!("Unknown NBT type: {}", typeid))
        }
    }
}