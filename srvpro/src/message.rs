//! srvpro's own messages.
//!
//! They come in two kinds: process-level events dispatched by the global handler chain, and
//! room-level events handled by the room that owns them.

use ygopro_derive::Message;

use ygopro_data::message::ctos;
use ygopro_data::message::stoc;

use crate::room::Player;

/// Initializes the server: the configuration, the plugins and the listeners all start here.
///
/// In theory, sending this message is all it takes to bring the whole server up.
///
/// Handled by the global handler chain ([`crate::SRVPRO_GLOBAL_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 1)]
pub struct Init {
    pub enable_plugins: hashbrown::HashSet<String>,
}

impl Default for Init {
    fn default() -> Self {
        Self { enable_plugins: crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS.iter().map(|plugin_name| plugin_name.to_string()).collect() }
    }
}

impl<State, Res> ygopro_handler::FromRequest<Message, State, Res> for &mut Init
where
    State: Send,
    Res: Send,
{
    fn from_request(bundle: &mut ygopro_handler::Bundle<Message, State, Res>) -> Option<Self> {
        if let Message::Init(inner) = &mut bundle.request {
            Some(unsafe { &mut *(inner as *mut Init) })
        } else {
            None
        }
    }
}

/// The configuration was replaced; handlers should reload their values.
///
/// Handled by the global handler chain ([`crate::SRVPRO_GLOBAL_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 2)]
pub struct ConfigurationChanged;

/// A plugin was enabled at runtime.
///
/// Handled by the global handler chain ([`crate::SRVPRO_GLOBAL_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 3)]
pub struct PluginEnabled {
    pub module_name: String,
}

/// A plugin was disabled at runtime.
///
/// Handled by the global handler chain ([`crate::SRVPRO_GLOBAL_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 4)]
pub struct PluginDisabled {
    pub module_name: String,
}

/// A new game client connected to the server.
///
/// Handled by the global handler chain ([`crate::SRVPRO_GLOBAL_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 11)]
pub struct ClientConnect {
    pub player: Player,
}

impl<State, Res> ygopro_handler::FromRequest<Message, State, Res> for &mut ClientConnect
where
    State: Send,
    Res: Send,
{
    fn from_request(bundle: &mut ygopro_handler::Bundle<Message, State, Res>) -> Option<Self> {
        if let Message::ClientConnect(inner) = &mut bundle.request {
            Some(unsafe { &mut *(inner as *mut ClientConnect) })
        } else {
            None
        }
    }
}

/// A game client connection was refused before joining any room.
///
/// Handled by the global handler chain ([`crate::SRVPRO_GLOBAL_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 14)]
pub struct ClientRefused;

/// A player asks to join a room; `position_sender` reports the assigned seat.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 101)]
pub struct PlayerJoin {
    pub player: Option<Player>,
    pub position_sender: Option<tokio::sync::oneshot::Sender<usize>>,
}

impl<Extra, State, Res> ygopro_handler::FromRequest<ygopro_handler::extract::Request<Message, Extra>, State, Res> for &mut PlayerJoin
where Extra: Send, State: Send, Res: Send,
{
    fn from_request(bundle: &mut ygopro_handler::Bundle<ygopro_handler::extract::Request<Message, Extra>, State, Res>) -> Option<Self> {
        if let Message::PlayerJoin(inner) = &mut bundle.request.message {
            Some(unsafe { &mut *(inner as *mut PlayerJoin) })
        } else {
            None
        }
    }
}

impl<State, Res> ygopro_handler::FromRequest<Message, State, Res> for &mut PlayerJoin
where State: Send, Res: Send,
{
    fn from_request(bundle: &mut ygopro_handler::Bundle<Message, State, Res>) -> Option<Self> {
        if let Message::PlayerJoin(inner) = &mut bundle.request {
            Some(unsafe { &mut *(inner as *mut PlayerJoin) })
        } else {
            None
        }
    }
}

/// A player left the room.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 102)]
pub struct ClientLeave {
    pub position: usize,
}

impl<Extra, State, Res> ygopro_handler::FromRequest<ygopro_handler::extract::Request<Message, Extra>, State, Res> for &mut ClientLeave
where Extra: Send, State: Send, Res: Send,
{
    fn from_request(bundle: &mut ygopro_handler::Bundle<ygopro_handler::extract::Request<Message, Extra>, State, Res>) -> Option<Self> {
        if let Message::ClientLeave(inner) = &mut bundle.request.message {
            Some(unsafe { &mut *(inner as *mut ClientLeave) })
        } else {
            None
        }
    }
}

impl<State, Res> ygopro_handler::FromRequest<Message, State, Res> for &mut ClientLeave
where State: Send, Res: Send,
{
    fn from_request(bundle: &mut ygopro_handler::Bundle<Message, State, Res>) -> Option<Self> {
        if let Message::ClientLeave(inner) = &mut bundle.request {
            Some(unsafe { &mut *(inner as *mut ClientLeave) })
        } else {
            None
        }
    }
}

/// A player decided his room going to.
///
/// Never dispatched: it is stored in [`crate::room::Player::states`] to remember which room
/// the player chose.
#[derive(Debug, Message)]
#[message(srvpro, flag = 109)]
pub struct PlayerMove {
    pub room_name: String
}

/// A room is created. 
/// 
/// Room is an empty struct in srvpro. The real ygopro instance (provider) is not created yet.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 110)]
pub struct CreateRoom;

/// A ygopro instance (provider) is created and ready to use.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 111)]
pub struct CreateProvider;

/// The ygopro instance (provider) failed to start.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 112)]
pub struct ProviderFail;

/// A duelist's life points changed.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 150)]
pub struct LPChanged;

/// A client message injected into the room; `target` restricts it to one player.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 201)]
pub struct DirectCTOS {
    pub message: ctos::Message,
    pub target: Option<usize>,
}

/// A server message injected into the room; `target` restricts it to one player.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 202)]
pub struct DirectSTOC {
    pub message: stoc::Message,
    pub target: Option<usize>,
}

/// A plain text notice for the room.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 203)]
pub struct ServerMessage {
    pub color: ygopro_data::constants::Color,
    pub message: String,
    pub target: Option<usize>,
}


/// The ygopro instance (provider) terminated.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 254)]
pub struct ProviderTerminate;

/// The process is asked to stop.
///
/// Handled by the room handler chain ([`crate::room::SRVPRO_HANDLERS`]).
#[derive(Debug, Message)]
#[message(srvpro, flag = 255)]
pub struct Terminate;

macro_rules! generate_enum {
    ($($message_name:ident = $message_flag:literal),*) => {
        #[doc = "Every srvpro event, keyed by the numbers declared in `generate_enum!`."]
        #[derive(Debug)]
        pub enum Message {
            $($message_name($message_name)),*
        }

        impl ygopro_data::message::PureMessage for Message {}

        impl ygopro_handler::MessageKey<u8> for Message {
            fn message_key(&self) -> u8 {
                match self {
                    $(Message::$message_name(_) => $message_flag),*
                }
            }
        }

        $(
            impl From<$message_name> for Message {
                fn from(value: $message_name) -> Self {
                    Message::$message_name(value)
                }
            }

            impl TryFrom<Message> for $message_name {
                type Error = ygopro_data::message::Error;

                fn try_from(value: Message) -> Result<Self, Self::Error> {
                    match value {
                        Message::$message_name(value) => Ok(value),
                        _ => Err(ygopro_data::message::Error::WrongType)
                    }
                }
            }

            impl $message_name {
                pub fn into_message(self) -> Message {
                    self.into()
                }
            }

            impl<State, Res> ygopro_handler::FromRequest<Message, State, Res> for &$message_name
            where
                State: Send,
                Res: Send,
            {
                fn from_request(bundle: &mut ygopro_handler::Bundle<Message, State, Res>) -> Option<Self> {
                    if let Message::$message_name(inner) = &bundle.request {
                        Some(unsafe { &*(inner as *const $message_name) })
                    } else {
                        None
                    }
                }
            }
        )*
    };
}

generate_enum!(
    Init = 1,
    ConfigurationChanged = 2,
    PluginEnabled = 3,
    PluginDisabled = 4,
    ClientConnect = 11,
    ClientRefused = 14,
    PlayerJoin = 101,
    ClientLeave = 102,
    PlayerMove = 109,
    CreateRoom = 110,
    CreateProvider = 111,
    ProviderFail = 112,
    LPChanged = 150,
    DirectCTOS = 201,
    DirectSTOC = 202,
    ServerMessage = 203,
    ProviderTerminate = 254,
    Terminate = 255
);
