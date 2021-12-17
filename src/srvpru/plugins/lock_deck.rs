use crate::srvpru::RoomSelector;

set_configuration! {
    get_deck: String,
    selector: RoomSelector
}

enum LockLevel {
    OfferDeck,
    SkipWait
}

pub fn init() -> anyhow::Result<()> {
    Ok(())
}

fn register_handlers() {

}