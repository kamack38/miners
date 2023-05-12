use miners_protocol::handler::PacketHandler;

use crate::{client::ClientPacketHandler, define_events};

#[derive(Clone)]
pub struct ChatHandler;

impl ClientPacketHandler for ChatHandler {
    fn handle(&self, client: &mut crate::client::MinecraftClient, packet: &miners_protocol::packet::RawPacket) {
        if client.socket.state != miners_protocol::ConnectionState::Play {
            return;
        }

        let mut packet = packet.clone();
        if packet.id == 0x33 {
            if packet.read_bool() { // Signature
                let sl = packet.read_varint();
                let _signature = packet.read_bytes(sl as usize);
            }

            let sender = packet.read_uuid();
            let hsl = packet.read_varint();
            let _header_signature = packet.read_bytes(hsl as usize);

            let message = packet.read_string();

            log::debug!(target: "miners-client", "Chat message received: {}", message);
            client.emit(ChatMessageEvent {
                message: ChatMessage {
                    source: ChatMessageSource::Player(sender),
                    message: FormattedChatMessage::from_plain(message.clone()), // TODO: Use formatted message
                    plain_message: message,
                }
            });
        } else {
            let message = packet.read_string();
            
            let message: FormattedChatMessage = message.into();
            let plain_message = message.text.clone();
            client.emit(ChatMessageEvent {
                message: ChatMessage {
                    source: ChatMessageSource::System,
                    message,
                    plain_message,
                }
            });
        }
    }

    fn ids(&self) -> &'static [i32] {
       &[0x33, 0x62]
    }
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub source: ChatMessageSource,
    pub message: FormattedChatMessage,
    pub plain_message: String,
}

fn default_font() -> String {
    String::from("minecraft:uniform")
}

fn default_color() -> String {
    String::from("minecraft:white")
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FormattedChatMessage {
    #[serde(default)]
    pub text: String,

    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underlined: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub obfuscated: bool,
    
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default = "default_color")]
    pub color: String,

    #[serde(default)]
    pub extra: Vec<FormattedChatMessage>,
}

impl Default for FormattedChatMessage {
    fn default() -> Self {
        FormattedChatMessage {
            text: String::new(),
            bold: false,
            italic: false,
            underlined: false,
            strikethrough: false,
            obfuscated: false,
            font: default_font(),
            color: default_color(),
            extra: Vec::new(),
        }
    }
}

impl From<String> for FormattedChatMessage {
    fn from(text: String) -> Self {
        serde_json::from_str(&text).unwrap()
    }
}

impl FormattedChatMessage {
    pub fn from_plain(text: String) -> Self {
        FormattedChatMessage {
            text,
            ..Default::default()
        }
    }
}

define_events!(ChatMessageEvent (message: ChatMessage));

#[derive(Debug, Clone)]
pub enum ChatMessageSource {
    Player(u128),
    System,
}