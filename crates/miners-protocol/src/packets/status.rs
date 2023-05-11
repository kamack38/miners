use serde::Deserialize;

use crate::packet::RawPacket;

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub version: Version,
    pub players: Players,
    pub description: Description,
    pub favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Deserialize)]
pub struct Players {
    pub max: i32,
    pub online: i32,
    pub sample: Option<Vec<Player>>,
}

#[derive(Debug, Deserialize)]
pub struct Player {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Description {
    pub text: String,
}

impl From<RawPacket> for StatusResponse {
    fn from(mut packet: RawPacket) -> Self {
        let json = packet.read_string();
        serde_json::from_str(&json).unwrap()
    }
}