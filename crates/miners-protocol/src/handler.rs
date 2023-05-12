use std::collections::BTreeMap;

use crate::{packet::RawPacket, RawMinecraftSocket, packets::login::LoginSuccessPacket};

/// Represents a packet handler
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

/// Structure responsible for managing packet handlers and handling packets
pub struct PacketHandlerManager {
    // Each packet id is mapped to a handler
    handlers: BTreeMap<i32, Box<dyn PacketHandler + Send + Sync>>,
    // Fallback handler for packets that don't have a handler registered
    fallback_handler: Option<Box<dyn PacketHandler + Send + Sync>>,
}

impl PacketHandlerManager {
    pub fn new() -> PacketHandlerManager {
        PacketHandlerManager {
            handlers: BTreeMap::new(),
            fallback_handler: None,
        }
    }

    /// Register new packet handler
    pub fn register(&mut self, handler: Box<dyn PacketHandler + Send + Sync>) {
        self.handlers.insert(handler.id(), handler);
    }

    /// Unregister all current handlers
    pub fn unregister_all(&mut self) {
        self.handlers.clear();
    }

    /// Set fallback handler
    pub fn register_fallback(&mut self, handler: Box<dyn PacketHandler + Send + Sync>) {
        self.fallback_handler = Some(handler);
    }

    /// Handle a packet
    pub fn handle(&self, connection: &mut RawMinecraftSocket,packet: RawPacket) -> Result<(), crate::PacketError> {
        // If there is a handler for this packet, handle it
        if let Some(handler) = self.handlers.get(&packet.id) {
            handler.handle(connection, packet).map_err(|e| crate::PacketError::text(format!("Handler error: {:?}", e)))?;
            return Ok(());
        }

        // Else, if there is a fallback handler, handle it
        if let Some(handler) = &self.fallback_handler {
            handler.handle(connection, packet).map_err(|e| crate::PacketError::text(format!("Handler error: {:?}", e)))?;
        } else {
            // Else, return an error
            return Err(crate::PacketError::text("No fallback handler found for packet".to_string()));
        }

        Ok(())
    }
}

// ====< Basic Handlers >====

/// Handles compression request packets (0x03) which are sent by the server when compression is enabled
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

/// Handles login success packets (0x02) which are sent by the server when login is successful
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
        connection.uuid = login_success_packet.uuid;

        Ok(())
    }
}

/// There is another packet after login success packet (0x02), this one is sent when server transitions to play state (0x25)
pub struct LoginPlayHandler;

impl PacketHandler for LoginPlayHandler {
    fn id(&self) -> i32 {
        0x25
    }

    fn handle(&self, connection: &mut RawMinecraftSocket, packet: RawPacket) -> Result<(), HandlerError> {
        // This can only be called if player is already in play state
        if connection.state != crate::ConnectionState::Play {
            return Err(HandlerError::BadState);
        }

        // Decode packet
        let login_play_packet = crate::packets::login::LoginPlayPacket::from(packet);
        
        // Print debug info about this packet
        log::debug!(target: "miners-protocol", "Login play packet received: {:#?}", login_play_packet);

        // Throw an error to exit the loop of handle_packets
        Err(HandlerError::ExitRequested)
    }
}