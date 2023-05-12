//! # miners-protocol
//! Currently all packets are implemented according to [1.19 protocol](https://wiki.vg/index.php?title=Protocol&oldid=17873)

use std::{sync::{Arc, Mutex}, net::TcpStream, io::Write, fmt::Debug};

use packet::{IntoPacket, RawPacket};
use packets::{handshake::HandshakePacket, status::StatusResponse, EmptyPacket};
use serde::Deserialize;

use crate::packets::login::LoginStartPacket;

pub mod packet;
pub mod handler;
pub mod packets;
pub mod utils;

/// Represents a raw minecraft socket for basic packet handling and sending
pub struct RawMinecraftSocket {
    pub socket: Arc<Mutex<TcpStream>>,
    pub host: (String, u16),
    pub compression_threshold: i32,
    pub handler_manager: Arc<Mutex<handler::PacketHandlerManager>>,
    pub state: ConnectionState,
    pub protocol_version: i32,
    pub uuid: u128,
}

impl Debug for RawMinecraftSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawMinecraftSocket")
            .field("host", &self.host)
            .field("compression_threshold", &self.compression_threshold)
            .field("state", &self.state)
            .field("protocol_version", &self.protocol_version)
            .finish()
    }
}

/// Represents the current connection state according to the protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Play,
}

/// Represents a client configuration for connecting to the server
#[derive(Debug, Clone)]
pub struct LoginConfig {
    pub username: String,
    pub host: String,
    pub port: u16,
}

impl Default for LoginConfig {
    fn default() -> Self {
        LoginConfig {
            username: String::from("miners_client"),
            host: String::from("localhost"),
            port: 25565,
        }
    }
}

impl RawMinecraftSocket {
    /// Creates a new socket from a TcpStream
    pub fn new(socket: Arc<Mutex<TcpStream>>) -> RawMinecraftSocket {
        RawMinecraftSocket {
            socket,
            host: (String::new(), 0),
            compression_threshold: -1,
            handler_manager: Arc::new(Mutex::new(handler::PacketHandlerManager::new())),
            state: ConnectionState::Handshake,
            protocol_version: -1,
            uuid: 0,
        }
    }

    /// Creates a new socket from host and port
    pub fn from_host(host: &str, port: u16) -> std::io::Result<RawMinecraftSocket> {
        let socket = Arc::new(Mutex::new(TcpStream::connect((host, port))?));
        Ok(RawMinecraftSocket {
            host: (host.to_string(), port),
            ..Self::new(socket)
        })
    }

    /// Disconnects from the server
    pub fn disconnect(&self) -> std::io::Result<()> {
        self.socket.lock().unwrap().shutdown(std::net::Shutdown::Both)
    }

    /// Connects to the server executing full handshake and login
    pub fn login(config: LoginConfig) -> Result<RawMinecraftSocket, PacketError> {
        // Get server info
        let socket = Self::from_host(&config.host, config.port).map_err(|_| PacketError {
            translate: String::from("miners.error.login.failed"),
            with: vec![String::from("Failed to connect to server")],
        })?;
        socket.send_packet(HandshakePacket::new_ping(config.host.clone(), config.port)).unwrap();
        socket.send_packet(EmptyPacket(0)).unwrap();

        let status = StatusResponse::from(socket.wait_for_packet().unwrap());
        log::debug!(target: "miners-protocol", "Server status: {:?}", status);

        // Login
        let mut socket = Self::from_host(&config.host, config.port).map_err(|_| PacketError {
            translate: String::from("miners.error.login.failed"),
            with: vec![String::from("Failed to connect to server")],
        })?;
        socket.protocol_version = status.version.protocol;

        // Add handlers
        socket.register_handler(Box::new(handler::SetCompressionHandler));
        socket.register_handler(Box::new(handler::LoginSuccessHandler));
        socket.register_handler(Box::new(handler::LoginPlayHandler));

        socket.send_packet(HandshakePacket::new_login(
            config.host,
            config.port
        )).unwrap();
        socket.state = ConnectionState::Login; // Change state to login

        socket.send_packet(LoginStartPacket::new(config.username)).unwrap();

        socket.handle_packets().ok();
        socket.handler_manager.lock().unwrap().unregister_all(); // Unregister all handlers
        Ok(socket)
    }

    pub fn send_packet(&self, packet: impl IntoPacket) -> std::io::Result<()> {
        // TODO: Implement encryption
        let packet = packet.into_packet(self.protocol_version);
        let mut socket = self.socket.lock().unwrap();
        
        // Prepend packet with id
        let mut new_packet = RawPacket::empty(packet.id);
        new_packet.write_varint(packet.id);
        new_packet.write_bytes(packet.data);

        let new_packet_len = new_packet.data.len();

        // Prepend packet with length
        let mut length_packet = RawPacket::empty(0);

        // Compression
        if self.compression_threshold > 0 {
            if new_packet_len >= self.compression_threshold as usize {
                let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(&new_packet.data)?;
                let compressed = e.finish()?;

                let mut new_length_packet = RawPacket::empty(0);
                new_length_packet.write_varint(new_packet_len as i32);
                new_length_packet.write_bytes(compressed);

                length_packet.write_varint(new_length_packet.data.len() as i32);
                length_packet.write_bytes(new_length_packet.data);
            } else {
                length_packet.write_varint(new_packet_len as i32 + 1);
                length_packet.write_varint(0); // Data length of 0
                length_packet.write_bytes(new_packet.data);
            }
        } else {
            length_packet.write_varint(new_packet_len as i32);
            length_packet.write_bytes(new_packet.data.clone());
        }

        socket.write_all(&length_packet.data)
    }

    /// Register a handler for packets
    pub fn register_handler(&mut self, handler: Box<dyn handler::PacketHandler + Send + Sync>) {
        self.handler_manager.lock().unwrap().register(handler);
    }

    /// Handle one packet with handlers
    fn handle(&mut self, packet: RawPacket) -> Result<(), PacketError> {
        let handler_manager_clone = self.handler_manager.clone();
        let handler_manager = handler_manager_clone.lock().unwrap();
        handler_manager.handle(self, packet)?;
        Ok(())
    }

    /// Handle packets indefinitely until error occurs in handler or packet reading
    pub fn handle_packets(&mut self) -> Result<(), PacketError> {
        loop {
            let packet = self.wait_for_packet()?;
            self.handle(packet)?;
        }
    }

    /// Handle packets indefinitely ignoring errors in handlers
    pub fn handle_packets_all(&mut self) -> Result<(), PacketError> {
        loop {
            let packet = self.wait_for_packet()?;
            self.handle(packet).ok(); // Ignore errors
        }
    }

    /// Handle one packet
    pub fn handle_packets_once(&mut self) -> Result<(), PacketError> {
        let packet = self.wait_for_packet()?;
        self.handle(packet)?;
        Ok(())
    }

    /// Check if there is a packet available to read, then if there is read it, otherwise return Error with text "No data to read"
    pub fn expect_packet(&self) -> Result<RawPacket, PacketError> {
        let socket = self.socket.clone();
        let mut socket = socket.lock().unwrap();
        let packet = RawPacket::read_from_socket(&mut *socket, self.compression_threshold, true)?;
        Ok(packet)
    }

    /// Wait for a packet to be available to read, then read it
    pub fn wait_for_packet(&self) -> Result<RawPacket, PacketError> {
        let socket = self.socket.clone();
        let mut socket = socket.lock().unwrap();
        let packet = RawPacket::read_from_socket(&mut *socket, self.compression_threshold, false)?;
        
        // Check for error
        let mut pc = packet.clone();
        match pc.try_read_string() {
            Some(es) => {
                if let Ok(error) = serde_json::from_str::<PacketError>(&es) {
                    return Err(error);
                } else {
                    return Ok(packet);
                }
            },
            None => {
                return Ok(packet);
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PacketError {
    translate: String,
    with: Vec<String>,
}

impl PacketError {
    pub fn new(translate: String, with: Vec<String>) -> PacketError {
        PacketError {
            translate,
            with,
        }
    }

    /// Creates a new packet error with translate text
    pub fn text(text: String) -> PacketError {
        PacketError {
            translate: text,
            with: vec![],
        }
    }

    /// Gets the translate text of the error
    pub fn get_text(&self) -> String {
        self.translate.clone()
    }
}