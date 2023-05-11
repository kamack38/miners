use std::{collections::HashMap, any::{TypeId, Any}, fmt::Debug, sync::Mutex, rc::Rc};

use crate::client::MinecraftClient;
pub mod basic;

pub trait ClientEvent: Clone {}

pub struct ClientEventDispatcher {
    handlers: Rc<Mutex<HashMap<TypeId, Vec<Rc<Mutex<dyn ClientEventHandler>>>>>>,
    handlers_once: Rc<Mutex<HashMap<TypeId, Vec<Rc<Mutex<dyn ClientEventHandler>>>>>>,

    pub(crate) event_queue: Mutex<Vec<Box<dyn Any>>>,
}

impl Debug for ClientEventDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientEventDispatcher")
            .field("handlers", &self.handlers.lock().unwrap().keys().map(|k| k.to_owned()).collect::<Vec<TypeId>>())
            .finish()
    }
}

impl ClientEventDispatcher {
    pub fn new() -> ClientEventDispatcher {
        ClientEventDispatcher {
            handlers: Rc::new(Mutex::new(HashMap::new())),
            handlers_once: Rc::new(Mutex::new(HashMap::new())),

            event_queue: Mutex::new(Vec::new()),
        }
    }

    pub fn register_handler<E: ClientEvent + 'static, F: Fn(&mut MinecraftClient, &E) + 'static>(&mut self, f: F) {
        let mut handlers = self.handlers.lock().unwrap();

        let handler = Rc::new(Mutex::new(ClientEventHandlerFunction {
            f,
            marker: std::marker::PhantomData,
        }));
        let type_id = TypeId::of::<E>();
        if let Some(handlers) = handlers.get_mut(&type_id) {
            handlers.push(handler);
        } else {
            handlers.insert(type_id, vec![handler]);
        }
    }

    pub fn register_handler_once<E: ClientEvent + 'static, F: Fn(&mut MinecraftClient, &E) + 'static>(&mut self, f: F) {
        let mut handlers_once = self.handlers_once.lock().unwrap();

        let handler = Rc::new(Mutex::new(ClientEventHandlerFunction {
            f,
            marker: std::marker::PhantomData,
        }));
        let type_id = TypeId::of::<E>();
        if let Some(handlers) = handlers_once.get_mut(&type_id) {
            handlers.push(handler);
        } else {
            handlers_once.insert(type_id, vec![handler]);
        }
    }

    pub fn queue(&mut self, event: Box<dyn Any>) {
        self.event_queue.lock().unwrap().push(event);
    }

    pub fn dispatch_all(client: &mut MinecraftClient) {
        let queue = { client.event_dispatcher.event_queue.lock().unwrap().drain(..).collect::<Vec<Box<dyn Any>>>() };
        for event in queue {
            Self::dispatch(client, event)
        }
    }

    pub fn dispatch(client: &mut MinecraftClient, event: Box<dyn Any>) {
        // TODO: Make this more efficient
        let handlers = { client.event_dispatcher.handlers.lock().unwrap().clone() };
        let handlers_once = { client.event_dispatcher.handlers_once.lock().unwrap().clone() };

        let type_id = (*event).type_id();
        if let Some(handlers) = handlers.get(&type_id) {
            for handler in handlers {
                handler.lock().unwrap().handle(client, &event);
            }
        }

        if let Some(handlers) = handlers_once.get(&type_id) {
            for handler in handlers {
                handler.lock().unwrap().handle(client, &event);
            }
        }

        // Remove all handlers that are only supposed to be called once
        client.event_dispatcher.handlers_once.lock().unwrap().remove(&type_id);
    }
}

pub trait ClientEventHandler {
    fn handle(&self, client: &mut MinecraftClient, event: &Box<dyn Any>);
}

pub struct ClientEventHandlerFunction<E: ClientEvent + 'static, F> {
    f: F,
    marker: std::marker::PhantomData<E>,
}

impl<E: ClientEvent + 'static, F: Fn(&mut MinecraftClient, &E)> ClientEventHandler for ClientEventHandlerFunction<E, F> {
    fn handle(&self, client: &mut MinecraftClient, event: &Box<dyn Any>) {
        let event = event.downcast_ref::<E>().unwrap();
        (self.f)(client, event);
    }
}

#[macro_export]
macro_rules! define_non_arg_events {
    ($($event:ident),*) => {
        $(
            #[derive(Clone)]
            pub struct $event;

            impl crate::events::ClientEvent for $event {}
        )*
    }
}

#[macro_export]
macro_rules! define_events {
    ($($event:ident($($arg:ident: $arg_type:ty),*)),*) => {
        $(
            #[derive(Clone)]
            pub struct $event {
                $(pub $arg: $arg_type),*
            }

            impl crate::events::ClientEvent for $event {}
        )*
    }
}