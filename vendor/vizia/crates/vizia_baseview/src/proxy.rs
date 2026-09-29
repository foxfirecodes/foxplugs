use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use vizia_core::context::EventProxy;
use vizia_core::events::Event;

#[derive(Clone, Default)]
pub(crate) struct BaseviewProxy(Arc<Mutex<VecDeque<Event>>>);

impl BaseviewProxy {
    pub(crate) fn pop(&self) -> Option<Event> {
        self.0.lock().unwrap().pop_front()
    }
}

impl EventProxy for BaseviewProxy {
    fn send(&self, event: Event) -> Result<(), ()> {
        self.0.lock().unwrap().push_back(event);
        Ok(())
    }
    fn make_clone(&self) -> Box<dyn EventProxy> {
        Box::new(self.clone())
    }
}
