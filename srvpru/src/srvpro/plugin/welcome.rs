use std::sync::Arc;

use parking_lot::Mutex;
use serde::Deserialize;
use ygopro::message::client_to_server::JoinGame;

use crate::srvpro::Player;
use crate::srvpro::Configuration;

#[derive(Configuration, Deserialize)]
#[serde(default)]
pub struct WelcomeConfiguration {
   welcome_message: String
}

impl Default for WelcomeConfiguration {
    fn default() -> Self {
        Self { welcome_message: "Srvpru Server".to_string() }
    }
}

#[after(JoinGame)]
async fn welcome(player: Arc<Mutex<Player>>) {
   let config = &WelcomeConfiguration::get();
   player.lock().send_chat_to_client(ygopro::constants::Colors::Green, config.welcome_message.clone()).await.ok();
}

#[test]
fn test_configuration() {
   let config = WelcomeConfiguration::get();
   println!("{:}", config.welcome_message);
}
