use std::{collections::BTreeMap, array::IntoIter, sync::{Arc, Mutex}};

use miners_protocol::{RawMinecraftSocket, LoginConfig, handler::PacketHandler, packet::RawPacket};

use crate::{events::{ClientEventDispatcher, ClientEvent, basic::SpawnEvent}, handlers::register_all_handlers};

pub struct MinecraftClient {
    pub socket: RawMinecraftSocket,
    pub username: String,

    pub(crate) event_dispatcher: ClientEventDispatcher, 
    pub(crate) client_packet_handlers: Arc<Mutex<BTreeMap<i32, Vec<Box<dyn ClientPacketHandler>>>>>,
}

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

impl MinecraftClient {
    pub fn new(client_config: ClientConfig) -> MinecraftClient {
        let socket = RawMinecraftSocket::login(LoginConfig {
            username: client_config.username.clone(),
            host: client_config.host.clone(),
            port: client_config.port,
        }).unwrap();

        let mut mc = MinecraftClient {
            socket,
            username: client_config.username,

            event_dispatcher: ClientEventDispatcher::new(),
            client_packet_handlers: Arc::new(Mutex::new(BTreeMap::new())),
        };

        mc.emit(SpawnEvent);

        mc
    }

    pub fn once<E: ClientEvent + 'static, F: Fn(&mut MinecraftClient, &E) + 'static>(&mut self, handler: F) {
        self.event_dispatcher.register_handler_once(handler);
    }

    pub fn on<E: ClientEvent + 'static, F: Fn(&mut MinecraftClient, &E) + 'static>(&mut self, handler: F) {
        self.event_dispatcher.register_handler(handler);
    }

    pub fn emit<E: ClientEvent + 'static>(&mut self, event: E) {
        self.event_dispatcher.queue(Box::new(event));
    }

    pub(crate) fn emit_now<E: ClientEvent + 'static>(&mut self, event: E) {
        self.event_dispatcher.queue(Box::new(event));
        ClientEventDispatcher::dispatch_all(self);
    }

    pub fn register_packet_handler<H: ClientPacketHandler + Clone + 'static>(&mut self, handler: H) {
        let handlers = self.client_packet_handlers.clone();
        let mut handlers = handlers.lock().unwrap();
        for id in handler.ids() {
            if let Some(v) = handlers.get_mut(&id) {
                v.push(Box::new(handler.clone()));
            } else {
                handlers.insert(*id, vec![Box::new(handler.clone())]);
            };
        }
    }

    pub fn start(&mut self) {
        register_all_handlers(self);
        loop {
            // TODO: Add tick event and a way to stop the handler
            ClientEventDispatcher::dispatch_all(self);
            let packet = self.socket.wait_for_packet();
            if packet.is_err() {
                log::error!(target: "miners-client", "Error while waiting for packet: {:?}", packet);
                break;
            }
            let packet = packet.unwrap();

            let handlers = self.client_packet_handlers.clone();
            if let Some(handlers) = handlers.lock().unwrap().get(&packet.id) {
                for handler in handlers {
                    handler.handle(self, &packet);
                }
            } else {
                // log::warn!(target: "miners-client", "Unhandled packet: {:?}", packet);
            };
        }
    }
}

pub trait ClientPacketHandler {
    fn handle(&self, client: &mut MinecraftClient, packet: &RawPacket);
    fn ids(&self) -> &'static [i32];
}