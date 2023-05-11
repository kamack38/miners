use crate::{packet::{IntoPacket, RawPacket}, utils::{location::Location, nbt::NBTType}};

#[derive(Debug, Clone)]
pub struct LoginStartPacket {
    pub username: String,
}

impl LoginStartPacket {
    pub fn new(username: String) -> LoginStartPacket {
        LoginStartPacket {
            username,
        }
    }
}

impl IntoPacket for LoginStartPacket {
    fn into_packet(self, _protocol_version: i32) -> crate::packet::RawPacket {
        let mut packet = crate::packet::RawPacket::empty(0);
        packet.write_string(&self.username);
        // TODO: Base this on protocol version
        packet.write_bool(false); // Has sig data
        packet.write_bool(false); // Has UUID
        packet
    }
}

#[derive(Debug, Clone)]
pub struct LoginPlayPacket {
    pub id: i32,
    pub is_hardcore: bool,
    pub gamemode: u8,
    pub previous_gamemode: i8,
    pub dimension_count: i32,
    pub dimension_names: Vec<String>,
    pub nbt_registry_codec: NBTType,
    pub dimension_type: String,
    pub dimension_name: String,
    pub hashed_seed: u64,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
    pub death_location: Option<Location>,
}

impl From<RawPacket> for LoginPlayPacket {
    fn from(mut value: RawPacket) -> Self {
        let id = value.read_int();
        let is_hardcore = value.read_bool();
        let gamemode = value.read_byte();
        let previous_gamemode = value.read_byte() as i8;
        let dimension_count = value.read_varint();
        let mut dimension_names = Vec::new();
        for _ in 0..dimension_count {
            dimension_names.push(value.read_string());
        }

        let nbt_registry_codec = NBTType::from_packet(&mut value).unwrap();
        let dimension_type = value.read_string();
        let dimension_name = value.read_string();
        let hashed_seed = value.read_ulong();
        let max_players = value.read_varint();
        let view_distance = value.read_varint();
        let simulation_distance = value.read_varint();
        let reduced_debug_info = value.read_bool();
        let enable_respawn_screen = value.read_bool();
        let is_debug = value.read_bool();
        let is_flat = value.read_bool();
        let death_location = if value.read_bool() {
            Some(Location::zero()) // TODO: Implement
        } else {
            None
        };

        LoginPlayPacket {
            id,
            is_hardcore,
            gamemode,
            previous_gamemode,
            dimension_count,
            dimension_names,
            nbt_registry_codec,
            dimension_type,
            dimension_name,
            hashed_seed,
            max_players,
            view_distance,
            simulation_distance,
            reduced_debug_info,
            enable_respawn_screen,
            is_debug,
            is_flat,
            death_location,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoginSuccessPacket {
    pub uuid: u128,
    pub username: String,
    pub properties: Vec<LoginSuccessProperty>,
}

#[derive(Debug, Clone)]
pub struct LoginSuccessProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

impl From<RawPacket> for LoginSuccessPacket {
    fn from(mut packet: RawPacket) -> Self {
        let uuid = packet.read_ulong() as u128 | ((packet.read_ulong() as u128) << 64);
        let username = packet.read_string();
        let properties_len = packet.read_varint();

        let mut properties = Vec::new();
        for _ in 0..properties_len {
            let name = packet.read_string();
            let value = packet.read_string();
            let has_signature = packet.read_bool();
            let signature = if has_signature {
                Some(packet.read_string())
            } else {
                None
            };
            properties.push(LoginSuccessProperty {
                name,
                value,
                signature,
            });
        }

        LoginSuccessPacket {
            uuid,
            username,
            properties,
        }
    }
}