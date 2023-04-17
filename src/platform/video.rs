mod video_impl {
    #[cfg(windows)]
    pub use crate::platform::win32::video::*;
    #[cfg(unix)]
    pub use crate::platform::unix::video::*;
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

pub fn get_size(width: &u32, height: &u32) {
    unsafe { video_impl::get_size(width, height) }
}

pub fn resized() -> bool {
    unsafe { video_impl::resized() }
}

pub fn focused() -> bool {
    unsafe { video_impl::focused() }
}
