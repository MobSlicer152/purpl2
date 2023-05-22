use ash::{extensions, vk};
use log::{debug, info};
use std::{ffi, mem, ptr};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const IDI_ICON1: u32 = 103;

const WINDOW_CLASS_NAME: &str = "PurplWindow";

const MAGIC: u32 = 0x11223344;

#[repr(C)]
pub struct State {
    magic: u32,
    window: HWND,
    title: String,
    width: u32,
    height: u32,
    resized: bool,
    focused: bool,
    closed: bool,
}

impl State {
    unsafe extern "system" fn wndproc(
        message_window: HWND,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        // Should probably work because pointer almost definitely points to _something_, and more than 4 bytes
        let self_ = match GetWindowLongPtrA(message_window, GWLP_USERDATA) {
            0 => { return DefWindowProcA(message_window, message, wparam, lparam); },
            addr => (addr as *mut Self).as_mut().unwrap()
        };

        if self_.magic != MAGIC {
            return DefWindowProcA(message_window, message, wparam, lparam);
        }

        if self_.window == 0 || message_window == self_.window {
            match message {
                WM_SIZE => {
                    let mut client_area: RECT = unsafe { mem::zeroed() };
    
                    GetClientRect(message_window, ptr::addr_of_mut!(client_area));
                    let new_width = (client_area.right - client_area.left) as u32;
                    let new_height = (client_area.bottom - client_area.top) as u32;
    
                    if new_width != self_.width || new_height != self_.height {
                        self_.resized = true;
                        info!(
                            "Window resized from {}x{} to {}x{}",
                            self_.width, self_.height, new_width, new_height
                        );
                    }
    
                    self_.width = new_width;
                    self_.height = new_height;
                    0
                }
                WM_ACTIVATEAPP => {
                    self_.focused = wparam != 0;
                    info!(
                        "Window {}",
                        if self_.focused {
                            "focused"
                        } else {
                            "unfocused"
                        }
                    );
                    0
                }
                WM_CLOSE => {
                    info!("Window closed");
                    self_.closed = true;
                    0
                }
                _ => DefWindowProcA(message_window, message, wparam, lparam),
            }
        } else {
            DefWindowProcA(message_window, message, wparam, lparam)
        }
    }
    
    unsafe fn register_wndclass() {
        debug!("Registering window class");
    
        let base_addr = GetModuleHandleA(ptr::null_mut());
        let mut window_class = WNDCLASSEXA {
            cbSize: mem::size_of::<WNDCLASSEXA>() as u32,
            lpfnWndProc: Some(Self::wndproc),
            hInstance: base_addr,
            hCursor: LoadCursorA(0, IDC_ARROW as *const u8),
            hIcon: LoadIconA(base_addr, IDI_ICON1 as *const u8),
            lpszClassName: WINDOW_CLASS_NAME.as_ptr(),
            ..mem::zeroed()
        };
        if RegisterClassExA(ptr::addr_of_mut!(window_class)) == 0 {
            let err = GetLastError();
            panic!(
                "Failed to register window class: error 0x{:X} ({})",
                err, err
            );
        }
    
        debug!("Window class registered");
    }
    
    unsafe fn init_wnd() -> (HWND, String, u32, u32) {
        let mut client_area: RECT = mem::zeroed();
        let base_addr = GetModuleHandleA(ptr::null_mut());
    
        client_area.left = 0;
        client_area.right = (GetSystemMetrics(SM_CXSCREEN) as f32 / 1.5) as i32;
        client_area.top = 0;
        client_area.bottom = (GetSystemMetrics(SM_CYSCREEN) as f32 / 1.5) as i32;
        AdjustWindowRect(
            ptr::addr_of_mut!(client_area),
            WS_OVERLAPPEDWINDOW,
            false as i32,
        );
        let width = (client_area.right - client_area.left) as u32;
        let height = (client_area.bottom - client_area.top) as u32;
    
        let title = format!(
            "{} v{}.{}.{} by {}",
            crate::GAME_NAME,
            crate::GAME_VERSION_MAJOR,
            crate::GAME_VERSION_MINOR,
            crate::GAME_VERSION_PATCH,
            crate::GAME_ORGANIZATION_NAME
        );
        debug!(
            "Creating {}x{} window titled {}",
            width, height, title
        );
    
        let window = CreateWindowExA(
            0,
            WINDOW_CLASS_NAME.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width as i32,
            height as i32,
            0,
            0,
            base_addr,
            ptr::null_mut(),
        );
        if window == 0 {
            let err = GetLastError();
            panic!("Failed to create window: error 0x{:X} {}", err, err);
        }
    
        GetClientRect(window, ptr::addr_of_mut!(client_area));
        let width = (client_area.right - client_area.left) as u32;
        let height = (client_area.bottom - client_area.top) as u32;
    
        debug!(
            "Successfully created window with handle 0x{:X}",
            window as usize
        );
    
        debug!("Showing window");
        ShowWindow(window, SW_SHOW);

        (window, title, width, height)
    }
    
    pub fn init() -> Self {
        info!("Windows video initialization started");
    
        let (window, title, width, height) = unsafe {
            Self::register_wndclass();
            Self::init_wnd()
        };
    
        info!("Windows video initialization succeeded");

        Self {
            magic: MAGIC,
            window,
            title,
            width,
            height,
            resized: false,
            focused: false,
            closed: false
        }
    }
    
    pub fn update(&mut self) -> bool {
        unsafe {
            let mut msg: MSG = mem::zeroed();
            SetWindowLongPtrA(self.window, GWLP_USERDATA, ptr::addr_of_mut!(*self) as isize);
            SetWindowPos(self.window, 0, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);

            while PeekMessageA(ptr::addr_of_mut!(msg), 0, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(ptr::addr_of_mut!(msg));
                DispatchMessageA(ptr::addr_of_mut!(msg));
            }
        }

        !self.closed
    }
    
    pub fn shutdown(&self) {
        info!("Windows video shutdown started");
    
        debug!("Destroying window");
        unsafe { DestroyWindow(self.window) };
    
        info!("Windows video shutdown succeeded");
    }
    
    pub fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    pub fn resized(&mut self) -> bool {
        let ret = self.resized;
        self.resized = false;
        ret
    }
    
    pub fn focused(&self) -> bool {
        self.focused
    }
    
    #[cfg(not(xbox))]
    pub fn create_vulkan_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        alloc_callbacks: Option<&vk::AllocationCallbacks>,
    ) -> vk::SurfaceKHR {
        unsafe {
            extensions::khr::Win32Surface::new(&entry, &instance)
                .create_win32_surface(
                    &vk::Win32SurfaceCreateInfoKHR {
                        hinstance: GetModuleHandleA(ptr::null_mut()) as *const ffi::c_void,
                        hwnd: self.window as *const ffi::c_void,
                        ..Default::default()
                    },
                    alloc_callbacks,
                )
                .unwrap_or_else(|err| panic!("Failed to create HWND surface: {}", err))
        }
    }
}
