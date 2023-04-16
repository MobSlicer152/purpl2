pub mod video;

#[cfg(windows)]
mod win32;
#[cfg(unix)]
mod unix;

mod platform_impl {
    #[cfg(windows)]
    pub use crate::platform::win32::*;
    #[cfg(unix)]
    pub use crate::platform::unix::*;
}

pub fn init() {
    unsafe { platform_impl::init() }
}

pub fn shutdown() {
    unsafe { platform_impl::shutdown() }
}
