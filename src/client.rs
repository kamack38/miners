use std::{collections::BTreeMap, sync::{Arc, Mutex, RwLock, RwLockWriteGuard, RwLockReadGuard}, time::Duration};

use miners_protocol::{RawMinecraftSocket, LoginConfig, packet::RawPacket};

use crate::{events::{ClientEventDispatcher, ClientEvent, basic::SpawnEvent}, handlers::register_all_handlers};

/// Minecraft client, used to connect to the server and handle events as well as packets
/// It is passed to event handlers as `ClientMutLock` (which is just `Arc<RwLock<MinecraftClient>>`)
/// 
/// # Example
/// ```rust
/// // Imports...
/// 
/// fn main() {
///    let mut client = MinecraftClient::new(ClientConfig::default());
///    client.once(|client: ClientMutLock, _: &SpawnEvent| {
///        let client = client.wl(); // Acquire write lock
///        client.send_chat_message("Hello, world!".to_string());
///        client.disconnect();
///    });
///    client.start();
/// }
/// ```      
pub struct MinecraftClient {
    pub socket: RawMinecraftSocket,
    pub username: String,
    pub uuid: u128,

    pub(crate) event_dispatcher: ClientEventDispatcher, 
    pub(crate) client_packet_handlers: BTreeMap<i32, Vec<Arc<Mutex<dyn ClientPacketHandler + Send + Sync + 'static>>>>,
}

/// Client configuration
/// Contains various options for the client such as username, host and port
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub username: String,
    pub host: String,
    pub port: u16,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            username: String::from("miners_client"),
            host: String::from("localhost"),
            port: 25565,
        }
    }
}

pub type ClientMutLock = Arc<RwLock<MinecraftClient>>;

impl Into<u128> for MinecraftClient {
    fn into(self) -> u128 {
        self.uuid
    }
}

impl MinecraftClient {
    /// Creates new client with specified config and connects to the server (blocking)
    pub fn new(client_config: ClientConfig) -> MinecraftClient {
        let socket = RawMinecraftSocket::login(LoginConfig {
            username: client_config.username.clone(),
            host: client_config.host.clone(),
            port: client_config.port,
        }).unwrap();

        let uuid = socket.uuid;
        let mut mc = MinecraftClient {
            socket,
            username: client_config.username,
            uuid,

            event_dispatcher: ClientEventDispatcher::new(),
            client_packet_handlers: BTreeMap::new(),
        };

        mc.emit(SpawnEvent);

        mc
    }

    /// Register new event handler that can be called only once (must be `Send + Sync` as it runs in a separate thread)
    pub fn once<E: ClientEvent + Send + Sync + 'static, F: Fn(ClientMutLock, &E) + Send + Sync + 'static>(&mut self, handler: F) {
        self.event_dispatcher.register_handler_once(handler);
    }

    /// Register new event handler (must be `Send + Sync` as it runs in a separate thread)
    pub fn on<E: ClientEvent + Send + Sync + 'static, F: Fn(ClientMutLock, &E) + Send + Sync + 'static>(&mut self, handler: F) {
        self.event_dispatcher.register_handler(handler);
    }

    /// Emits event to the client (can be handled using `on` or `once`)
    pub fn emit<E: ClientEvent + Send + Sync + 'static>(&mut self, event: E) {
        self.event_dispatcher.queue(Box::new(event));
    }

    /// Registers new raw packet handler (`ClientPacketHandler`)
    pub fn register_packet_handler<H: ClientPacketHandler + Send + Sync + 'static>(&mut self, handler: H) {
        let ids = handler.ids();
        let handler = Arc::new(Mutex::new(handler));
        for id in ids {
            if let Some(v) = self.client_packet_handlers.get_mut(&id) {
                v.push(handler.clone());
            } else {
                self.client_packet_handlers.insert(*id, vec![handler.clone()]);
            };
        }
    }

    /// Handle single packet asynchronously
    pub fn handle_packet(_self: Arc<RwLock<MinecraftClient>>, packet: RawPacket) {
        let handlers = {
            let _self = _self.read().unwrap();
            _self.client_packet_handlers.get(&packet.id).cloned()
        };
        if let Some(handlers) = handlers {
            let handlers = handlers.clone(); // Clone to avoid locking the mutex for too long
            for handler in handlers {
                handler.lock().unwrap().handle(_self.clone(), &packet);
            }
        }
    }

    /// Disconnects from the server and emits `DisconnectEvent`
    pub fn disconnect(&mut self) {
        // TODO: Stop all threads and stuff
        self.socket.disconnect().ok();
        self.emit(crate::events::basic::DisconnectEvent);
    }

    /// Starts listening for packets and dispatching events (blocking)
    pub fn start(mut self) {
        register_all_handlers(&mut self);
        let _self = Arc::new(RwLock::new(self));
        loop {
            // Dispatch events
            ClientEventDispatcher::dispatch_all(_self.clone());

            // Handle packets
            let packet = {
                loop {
                    let mut _self = _self.read().unwrap();
                    match _self.socket.expect_packet() {
                        Ok(packet) => break Ok(packet),
                        Err(e) => {
                            println!("{:?}", e);
                            if e.get_text() != "No data to read" {
                                log::error!(target: "miners-client", "Error receiving packet: {:?}", e);
                                break Err(e);
                            }
                        }
                    }
                    drop(_self); // Unlock mutex
                    std::thread::sleep(Duration::from_millis(10));
                }
            };

            if let Ok(packet) = packet {
                let _self = _self.clone();
                MinecraftClient::handle_packet(_self, packet);
            } else {
                log::error!(target: "miners-client", "Error receiving packet: {:?}", packet);
                break;
            }
        }
    }
}

pub trait ClientPacketHandler {
    fn handle(&self, client: ClientMutLock, packet: &RawPacket);
    fn ids(&self) -> &'static [i32];
}

pub trait ClientLockExt {
    /// Emits event to the client (equivalent to `self.write().unwrap().emit(e)`)
    fn emit(&self, e: impl ClientEvent + Send + Sync + 'static);
    
    /// Returns current connection state (equivalent to `self.read().unwrap().socket.state`)
    fn get_state(&self) -> miners_protocol::ConnectionState;

    /// Acquires write lock and returns it (equivalent to `self.write().unwrap()`)
    fn wl(&self) -> RwLockWriteGuard<MinecraftClient>;
    /// Acquires read lock and returns it (equivalent to `self.read().unwrap()`)
    fn rl(&self) -> RwLockReadGuard<MinecraftClient>;
}

/// ====< Some weird stuff to make life easier >====
pub(crate) trait ClientPrivateLockExt {
    fn emit_now(&self, e: impl ClientEvent + Send + Sync + 'static);
}

impl ClientPrivateLockExt for ClientMutLock {
    fn emit_now(&self, e: impl ClientEvent + Send + Sync + 'static) {
        self.emit(e);
        ClientEventDispatcher::dispatch_all(self.clone());
    }
}

impl ClientLockExt for ClientMutLock {
    fn emit(&self, e: impl ClientEvent + Send + Sync + 'static) {
        self.write().unwrap().emit(e);
    }

    fn get_state(&self) -> miners_protocol::ConnectionState {
        self.read().unwrap().socket.state
    }

    fn wl(&self) -> RwLockWriteGuard<MinecraftClient> {
        self.write().unwrap()
    }

    fn rl(&self) -> RwLockReadGuard<MinecraftClient> {
        self.read().unwrap()
    }
}