// ============================================================
// virtual_password
// ------------------------------------------------------------
//! User can take $ as password splitter.
// ============================================================

use crate::ygopro::message::ctos;
use crate::srvpru::Handler;
use crate::srvpru::CommonError;

pub fn init() -> anyhow::Result<()> {
    register_handlers();
    Ok(())
}

pub fn register_handlers() {
    Handler::before_message::<ctos::PlayerInfo, _>(11, "virtual_password", |context, message| Box::pin(async move {
        let name = context.get_string(&message.name, "name")?;
        if let Some(index) = name.as_str().rfind("$") {
            let actual_name = name[0..index].to_owned();
            let name = name.clone();
            if ! context.get_player().ok_or(CommonError::PlayerNotExist)?.lock().try_set_origin_name(name) { return Ok(false); }
            message.name = crate::ygopro::message::string::cast_to_fix_length_array(&actual_name);
            context.reserialize = true;
            context.set_parameter("name", actual_name);
        }
        Ok(false)
    })).register();
    Handler::register_handlers("virtual_password", crate::ygopro::message::Direction::CTOS, vec!["virtual_password"])
}