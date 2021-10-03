use crate::srvpru::processor::Handler;

pub struct Debugger;

impl Debugger {
    pub fn register_handlers() {
        let ctos_handler = Handler::new(0, |_context| true, |context| Box::pin(async move {
            trace!("CTOS Message -> {:?}", context.message_type.as_ref().unwrap());
            false
        }));

        let stoc_handler = Handler::new(0, |_context| true, |context| Box::pin(async move {
            trace!("STOC Message <- {:?}", context.message_type.as_ref().unwrap());
            false
        }));
        
        Handler::register_handler("ctos_debugger", ctos_handler);
        Handler::register_handler("stoc_debugger", stoc_handler);
    }


}