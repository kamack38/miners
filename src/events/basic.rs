use crate::define_non_arg_events;

define_non_arg_events!(SpawnEvent => "Emitted when the player spawns for the first time (on login)");
define_non_arg_events!(DeathEvent => "Emitted when player dies"); // This may change to include the death message
define_non_arg_events!(DisconnectEvent => "Emitted when client disconnects from the server (currently only when `.disconnect()` is called)");