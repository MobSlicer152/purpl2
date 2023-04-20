#[cfg(unix)]
mod video_impl {
    use crate::platform::unix::video;
    use std::sync::Mutex;

    static STATE: Mutex<Option<video::State>> = Mutex::new(None);
    pub fn init() {
        *STATE.lock().unwrap() = Some(video::State::init())
    }
    pub fn update() -> bool {
        STATE.lock().unwrap().as_mut().unwrap().update()
    }
    pub fn shutdown() {
        STATE.lock().unwrap().as_mut().unwrap().shutdown()
    }
    pub fn get_size() -> (u32, u32) {
        STATE.lock().unwrap().as_ref().unwrap().get_size()
    }
    pub fn resized() -> bool {
        STATE.lock().unwrap().as_mut().unwrap().resized()
    }
    pub fn focused() -> bool {
        STATE.lock().unwrap().as_ref().unwrap().focused()
    }
}
#[cfg(windows)]
mod video_impl {
    use crate::platform::win32::video;
    pub fn init() {
        unsafe { video::init() }
    }
    pub fn update() -> bool {
        unsafe { video::update() }
    }
    pub fn shutdown() {
        unsafe { video::shutdown() }
    }
    pub fn get_size() -> (u32, u32) {
        unsafe { video::get_size() }
    }
    pub fn resized() -> bool {
        unsafe { video::resized() }
    }
    pub fn focused() -> bool {
        unsafe { video::focused() }
    }
}

pub use video_impl::*;
