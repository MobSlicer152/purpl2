pub mod video;

// #[cfg(unix)]
// mod unix;
// #[cfg(windows)]
mod win32;

mod platform_impl {
    // #[cfg(unix)]
    // pub use crate::platform::unix::*;
    // #[cfg(windows)]
    pub use crate::platform::win32::*;
}

pub fn init() {
    unsafe { platform_impl::init() }
}

pub fn shutdown() {
    unsafe { platform_impl::shutdown() }
}
