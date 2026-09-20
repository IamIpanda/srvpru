use ygopro_derive::Message;

use ygopro_data::message::ctos;
use ygopro_data::message::stoc;

use crate::room::Player;

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

#[derive(Debug, Message)]
#[message(srvpro, flag = 109)]
pub struct PlayerMove {
    pub room_name: String
}

#[derive(Debug, Message)]
#[message(srvpro, flag = 110)]
pub struct CreateRoom;

#[derive(Debug, Message)]
#[message(srvpro, flag = 111)]
pub struct CreateProvider;

#[derive(Debug, Message)]
#[message(srvpro, flag = 112)]
pub struct ProviderFail;

#[derive(Debug, Message)]
#[message(srvpro, flag = 150)]
pub struct LPChanged;

#[derive(Debug, Message)]
#[message(srvpro, flag = 201)]
pub struct DirectCTOS {
    pub message: ctos::Message,
    pub target: Option<usize>,
}

#[derive(Debug, Message)]
#[message(srvpro, flag = 202)]
pub struct DirectSTOC {
    pub message: stoc::Message,
    pub target: Option<usize>,
}

#[derive(Debug, Message)]
#[message(srvpro, flag = 14)]
pub struct ClientRefused;

#[derive(Debug, Message)]
#[message(srvpro, flag = 1)]
pub struct Init;

#[derive(Debug, Message)]
#[message(srvpro, flag = 2)]
pub struct ConfigurationChanged;

#[derive(Debug, Message)]
#[message(srvpro, flag = 3)]
pub struct PluginEnabled {
    pub module_name: String,
}

#[derive(Debug, Message)]
#[message(srvpro, flag = 4)]
pub struct PluginDisabled {
    pub module_name: String,
}


#[derive(Debug, Message)]
#[message(srvpro, flag = 254)]
pub struct ProviderTerminate;

#[derive(Debug, Message)]
#[message(srvpro, flag = 255)]
pub struct Terminate;

macro_rules! generate_enum {
    ($($message_name:ident = $message_flag:literal),*) => {
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

            // impl From<$message_name> for crate::common::RequestEx {
            //     fn from(message: $message_name) -> Self {
            //         crate::common::RequestEx { message: message.into(), extra: () }
            //     }
            // }

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
    ProviderTerminate = 254,
    Terminate = 255
);
