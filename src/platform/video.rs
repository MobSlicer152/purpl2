mod video_impl {
    #[cfg(windows)]
    pub use crate::platform::win32::video::*;
}

pub fn init() {
    unsafe { video_impl::init() }
}

pub fn update() -> bool {
    unsafe { video_impl::update() }
}

pub fn shutdown() {
    unsafe { video_impl::shutdown() }
}
