mod video_impl {
    #[cfg(unix)]
    pub use crate::platform::unix::video::*;
    #[cfg(windows)]
    pub use crate::platform::win32::video::*;
}

use once_cell::sync::Lazy;
use std::sync::Mutex;
use video_impl::State;

static STATE: Lazy<Mutex<Option<State>>> = Lazy::new(|| Mutex::new(None));

pub fn init() {
    *STATE.lock().unwrap() = Some(State::init());
}

pub fn update() -> bool {
    STATE.lock().unwrap().as_mut().unwrap().update()
}

pub fn shutdown() {
    STATE.lock().unwrap().as_mut().unwrap().shutdown()
}

pub fn get_size() -> (u32, u32) {
    STATE.lock().unwrap().as_mut().unwrap().get_size()
}

pub fn resized() -> bool {
    STATE.lock().unwrap().as_mut().unwrap().resized()
}

pub fn focused() -> bool {
    STATE.lock().unwrap().as_mut().unwrap().focused()
}
