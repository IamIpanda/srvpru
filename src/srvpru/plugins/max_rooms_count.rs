use crate::srvpru::message::RoomCreated;
use crate::srvpru::Handler;


set_configuration! {
    #[serde(default)]
    max_rooms_count: usize
}

pub fn init() -> anyhow::Result<()> {
    load_configuration()?;
    register_handlers();
    Ok(())   
}

pub fn register_handlers() {
    Handler::before_message::<RoomCreated, _>(1, "max_rooms_count", |context, _| Box::pin(async move {
        let configuration = get_configuration();
        if configuration.max_rooms_count > 0 && crate::srvpru::ROOMS.read().len() >= configuration.max_rooms_count {
            context.block_message = true;
        }
        Ok(false)
    })).register_for_plugin("max_rooms_count");
}