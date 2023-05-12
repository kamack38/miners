use crate::{client::{ClientPacketHandler, ClientMutLock, ClientLockExt}, define_events};

/// Handler for basic chat messages
/// 
/// Handles both player and system messages (e.g. death messages)
#[derive(Clone)]
pub struct ChatHandler;

impl ClientPacketHandler for ChatHandler {
    fn handle(&self, client: ClientMutLock, packet: &miners_protocol::packet::RawPacket) {
        // Ensure that we are in play state
        if client.get_state() != miners_protocol::ConnectionState::Play {
            return;
        }

        // Clone packet to read it
        let mut packet = packet.clone();
        // If packet id is 0x33 (Player Chat Message), read additional data
        if packet.id == 0x33 {
            // ==< Header >==
            if packet.read_bool() { // Signature
                let sl = packet.read_varint();
                let _signature = packet.read_bytes(sl as usize);
            }

            let sender = packet.read_uuid();
            let hsl = packet.read_varint();
            let _header_signature = packet.read_bytes(hsl as usize);

            let message = packet.read_string();

            //? For now we ignore remaining data as we don't use it yet

            log::debug!(target: "miners-client", "Chat message received: {}", message);
            // Emit event with appropriate data
            client.emit(ChatMessageEvent {
                message: ChatMessage {
                    source: ChatMessageSource::Player(sender),
                    message: FormattedChatMessage::from_plain(message.clone()), // TODO: Use formatted message
                    plain_message: message,
                }
            });
        } else {
            // If it is not 0x33, then it is 0x62 (System Chat Message)
            let message = packet.read_string();
            
            let message: FormattedChatMessage = message.into();
            let plain_message = message.text.clone();
            
            //? There is no more data in this packet :)

            log::debug!(target: "miners-client", "System chat message received: {}", message.text);
            // Emit event with appropriate data
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

/// Represents a chat message (both player and system)
/// 
/// Contains message sender, formatted message and plain message
/// 
/// **Warning:** This may change drastically in the future
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub source: ChatMessageSource,
    pub message: FormattedChatMessage,
    pub plain_message: String,
}

/// Default font used in chat messages (Required by serde for default value)
fn default_font() -> String {
    String::from("minecraft:uniform")
}

/// Default color used in chat messages (Required by serde for default value)
fn default_color() -> String {
    String::from("minecraft:white")
}

/// Parsed version of formatted chat message (Like the one you pass to `/tellraw`)
/// 
/// **Warning:** This currently does not contain all fields that can be used in chat messages
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

define_events!(ChatMessageEvent (message: ChatMessage) => "Event emitted when chat message is received");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatMessageSource {
    Player(u128),
    System,
}

impl From<&ClientMutLock> for ChatMessageSource {
    fn from(client: &ClientMutLock) -> Self {
        let rl = client.read().unwrap();
        ChatMessageSource::Player(rl.uuid.clone())
    }
}