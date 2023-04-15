pub mod video;

#[cfg(windows)]
mod win32;

mod platform_impl {
    #[cfg(windows)]
    pub use super::win32::*;
}

pub fn init() {
    unsafe { platform_impl::init() }
}

pub fn shutdown() {
    unsafe { platform_impl::shutdown() }
}
