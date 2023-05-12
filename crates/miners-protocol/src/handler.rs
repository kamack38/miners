use std::collections::BTreeMap;

use crate::{packet::RawPacket, RawMinecraftSocket, packets::login::LoginSuccessPacket};

pub trait PacketHandler {
    fn id(&self) -> i32;
    fn handle(&self, connection: &mut RawMinecraftSocket, packet: RawPacket) -> Result<(), HandlerError>; 
}

#[derive(Debug)]
pub enum HandlerError {
    BadState,
    ExitRequested,
    IOError(std::io::Error),
}

impl From<std::io::Error> for HandlerError {
    fn from(e: std::io::Error) -> Self {
        HandlerError::IOError(e)
    }
}

pub struct PacketHandlerManager {
    handlers: BTreeMap<i32, Box<dyn PacketHandler + Send + Sync>>,
    fallback_handler: Option<Box<dyn PacketHandler + Send + Sync>>,
}

impl PacketHandlerManager {
    pub fn new() -> PacketHandlerManager {
        PacketHandlerManager {
            handlers: BTreeMap::new(),
            fallback_handler: None,
        }
    }

    pub fn register(&mut self, handler: Box<dyn PacketHandler + Send + Sync>) {
        self.handlers.insert(handler.id(), handler);
    }

    pub fn unregister_all(&mut self) {
        self.handlers.clear();
    }

    pub fn register_fallback(&mut self, handler: Box<dyn PacketHandler + Send + Sync>) {
        self.fallback_handler = Some(handler);
    }

    pub fn handle(&self, connection: &mut RawMinecraftSocket,packet: RawPacket) -> Result<(), crate::PacketError> {
        if let Some(handler) = self.handlers.get(&packet.id) {
            handler.handle(connection, packet).map_err(|e| crate::PacketError::text(format!("Handler error: {:?}", e)))?;
            return Ok(());
        }

        if let Some(handler) = &self.fallback_handler {
            handler.handle(connection, packet).map_err(|e| crate::PacketError::text(format!("Handler error: {:?}", e)))?;
        } else {
            return Err(crate::PacketError::text("No fallback handler found for packet".to_string()));
        }

        Ok(())
    }
}

// ====< Basic Handlers >====
pub struct SetCompressionHandler;

impl PacketHandler for SetCompressionHandler {
    fn id(&self) -> i32 {
        0x03
    }

    fn handle(&self, connection: &mut RawMinecraftSocket, mut packet: RawPacket) -> Result<(), HandlerError> {
        if connection.state != crate::ConnectionState::Login {
            return Err(HandlerError::BadState);
        }

        let threshold = packet.read_varint();
        log::debug!(target: "miners-protocol", "Set compression packet received with threshold: {}", threshold);
        connection.compression_threshold = threshold;
        Ok(())
    }
}

pub struct LoginSuccessHandler;

impl PacketHandler for LoginSuccessHandler {
    fn id(&self) -> i32 {
        0x02
    }

    fn handle(&self, connection: &mut RawMinecraftSocket, packet: RawPacket) -> Result<(), HandlerError> {
        if connection.state != crate::ConnectionState::Login {
            return Err(HandlerError::BadState);
        }

        let login_success_packet = LoginSuccessPacket::from(packet);
        log::debug!(target: "miners-protocol", "Login success packet received: {:?}", login_success_packet);
        connection.state = crate::ConnectionState::Play;

        Ok(())
    }
}

pub struct LoginPlayHandler;

impl PacketHandler for LoginPlayHandler {
    fn id(&self) -> i32 {
        0x25
    }

    fn handle(&self, connection: &mut RawMinecraftSocket, packet: RawPacket) -> Result<(), HandlerError> {
        if connection.state != crate::ConnectionState::Play {
            return Err(HandlerError::BadState);
        }

        let login_play_packet = crate::packets::login::LoginPlayPacket::from(packet);
        log::debug!(target: "miners-protocol", "Login play packet received: {:#?}", login_play_packet);

        Err(HandlerError::ExitRequested)
    }
}