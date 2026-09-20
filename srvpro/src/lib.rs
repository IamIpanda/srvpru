pub mod configuration;
pub mod managers;
pub mod message;
pub mod mode;
pub mod plugin;
pub mod room;
pub mod server;

#[macro_use]
extern crate linkme;
#[macro_use]
extern crate ygopro_derive;

use std::sync::LazyLock;

use ygopro_handler::Bundle;
use ygopro_handler::MessageKey;
use ygopro_handler::Processor;
use ygopro_handler::TowerHandler;
pub use ygopro_handler::extract::Response;
use message as srvpro;

pub type GlobalHandler = TowerHandler<srvpro::Message, (), Response<srvpro::Message>>;

#[distributed_slice]
pub static SRVPRO_GLOBAL_HANDLERS: [fn() -> (u8, GlobalHandler)];

pub static GLOBAL_PROCESSOR: LazyLock<Processor<u8, srvpro::Message, (), Response<srvpro::Message>, GlobalHandler>> = LazyLock::new(|| {
    Processor::new_with_groups(&SRVPRO_GLOBAL_HANDLERS, &hashbrown::HashSet::new(), |handler| handler.module_name, |key| *key == 0)
});

pub async fn process(event: srvpro::Message) -> Response<srvpro::Message> {
    let key = event.message_key();
    GLOBAL_PROCESSOR.process_bundle(Bundle::new(event, (), Response::Continue), key).await.response
}

#[handler(srvpro::Init)]
#[register_to(SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_init() {
    ygopro::init();
}
