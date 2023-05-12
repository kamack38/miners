use std::{collections::HashMap, any::{TypeId, Any}, fmt::Debug, sync::{Mutex, Arc}};

use crate::client::{ClientMutLock};
pub mod basic;

pub trait ClientEvent {}

/// Structure for handling client events (such as `SpawnEvent`)
/// 
/// This is used internally by the client, but can also be used to register custom events.
/// Usage is pretty complex, so I recommend you to look at the source code of the default events.
pub struct ClientEventDispatcher {
    // Emm... I don't know how to explain this one so I'll just leave it as it is :P
    handlers: Arc<Mutex<HashMap<TypeId, Vec<Arc<Mutex<dyn ClientEventHandler + Send + Sync>>>>>>,
    handlers_once: Arc<Mutex<HashMap<TypeId, Vec<Arc<Mutex<dyn ClientEventHandler + Send + Sync>>>>>>,

    // Queue of events that are to be dispatched
    pub(crate) event_queue: Mutex<Vec<Box<dyn Any + Send + Sync>>>,
}

impl Debug for ClientEventDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientEventDispatcher")
            .field("handlers", &self.handlers.lock().unwrap().keys().map(|k| k.to_owned()).collect::<Vec<TypeId>>())
            .finish()
    }
}

impl ClientEventDispatcher {
    /// Creates a new `ClientEventDispatcher`
    pub fn new() -> ClientEventDispatcher {
        ClientEventDispatcher {
            handlers: Arc::new(Mutex::new(HashMap::new())),
            handlers_once: Arc::new(Mutex::new(HashMap::new())),

            event_queue: Mutex::new(Vec::new()),
        }
    }

    /// Registers a handler for an event.
    /// Event type is inferred from the function signature (so you don't have to specify it)
    /// 
    /// It is recommended to use closures for handlers, as they are easier to write and look neat.
    pub fn register_handler<E: ClientEvent + Send + Sync + 'static, F: Fn(ClientMutLock, &E) + Send + Sync + 'static>(&mut self, f: F) {
        // Acquire lock on handlers map
        let mut handlers = self.handlers.lock().unwrap();

        // Create handler
        let handler = Arc::new(Mutex::new(ClientEventHandlerFunction {
            f,
            marker: std::marker::PhantomData,
        }));
        // Get type id of the event
        let type_id = TypeId::of::<E>();
        // If there are already handlers for this event, add the new one to the list
        if let Some(handlers) = handlers.get_mut(&type_id) {
            handlers.push(handler);
        } else {
            // If there are no handlers for this event, create a new list and add the handler to it
            handlers.insert(type_id, vec![handler]);
        }
    }

    /// Registers a handler for an event that will be called only once.
    /// For more info see [`register_handler`](ClientEventDispatcher::register_handler)
    pub fn register_handler_once<E: ClientEvent + Send + Sync + 'static, F: Fn(ClientMutLock, &E) + Send + Sync + 'static>(&mut self, f: F) {
        // Acquire lock on handlers map
        let mut handlers_once = self.handlers_once.lock().unwrap();

        // Create handler
        let handler = Arc::new(Mutex::new(ClientEventHandlerFunction {
            f,
            marker: std::marker::PhantomData,
        }));
        // Get type id of the event
        let type_id = TypeId::of::<E>();
        
        // If there are already handlers for this event, add the new one to the list
        if let Some(handlers) = handlers_once.get_mut(&type_id) {
            handlers.push(handler);
        } else {
            // If there are no handlers for this event, create a new list and add the handler to it
            handlers_once.insert(type_id, vec![handler]);
        }
    }

    /// Queues an event to be dispatched later using [`dispatch_all`](ClientEventDispatcher::dispatch_all)
    pub fn queue(&mut self, event: Box<dyn Any + Send + Sync>) {
        self.event_queue.lock().unwrap().push(event);
    }

    /// Dispatches all events in the queue (each event is dispatched in a separate thread for non-blocking execution)
    pub fn dispatch_all(client: ClientMutLock) {
        // Drain the queue
        // And that's another reason why I love Rust :)
        let queue = { client.read().unwrap().event_dispatcher.event_queue.lock().unwrap().drain(..).collect::<Vec<Box<dyn Any + Send + Sync>>>() };
        
        // Dispatch each event in a separate thread
        for event in queue {
            let client = client.clone();
            std::thread::spawn(move || {
                Self::dispatch(client, event)
            });
        }
    }

    /// Dispatches a single event
    pub fn dispatch(client: ClientMutLock, event: Box<dyn Any>) {
        // TODO: Make this more efficient
        let handlers = { client.read().unwrap().event_dispatcher.handlers.lock().unwrap().clone() };
        let handlers_once = { client.read().unwrap().event_dispatcher.handlers_once.lock().unwrap().clone() };

        // Get type id of the event
        let type_id = (*event).type_id();
        
        // Call all handlers
        if let Some(handlers) = handlers.get(&type_id) {
            for handler in handlers {
                handler.lock().unwrap().handle(client.clone(), &event);
            }
        }

        // Call all handlers that are only supposed to be called once
        if let Some(handlers) = handlers_once.get(&type_id) {
            for handler in handlers {
                handler.lock().unwrap().handle(client.clone(), &event);
            }
        }

        // Remove all handlers that are only supposed to be called once
        client.write().unwrap().event_dispatcher.handlers_once.lock().unwrap().remove(&type_id);
    }
}

// ====< WARNING: Black magic ahead >====
// Everything below is responsible for allowing you to use closures as event handlers

pub trait ClientEventHandler {
    fn handle(&self, client: ClientMutLock, event: &Box<dyn Any>);
}

pub struct ClientEventHandlerFunction<E: ClientEvent + 'static, F> {
    f: F,
    marker: std::marker::PhantomData<E>,
}

impl<E: ClientEvent + 'static, F: Fn(ClientMutLock, &E)> ClientEventHandler for ClientEventHandlerFunction<E, F> {
    fn handle(&self, client: ClientMutLock, event: &Box<dyn Any>) {
        let event = event.downcast_ref::<E>().unwrap();
        (self.f)(client, event);
    }
}

#[macro_export]
macro_rules! define_non_arg_events {
    ($($event:ident => $doc:literal),*) => {
        $(
            #[doc = $doc]
            #[derive(Clone)]
            pub struct $event;

            impl crate::events::ClientEvent for $event {}
        )*
    }
}

#[macro_export]
macro_rules! define_events {
    ($($event:ident($($arg:ident: $arg_type:ty),*) => $doc:literal),*) => {
        $(
            #[doc = $doc]
            #[derive(Clone)]
            pub struct $event {
                $(pub $arg: $arg_type),*
            }

            impl crate::events::ClientEvent for $event {}
        )*
    }
}