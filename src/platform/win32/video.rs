use log::{debug, info};
use winapi::shared::windef;
use winapi::um::errhandlingapi;
use winapi::um::libloaderapi;
use winapi::um::winuser;

const IDI_ICON1: u32 = 103;

static mut WND: windef::HWND = std::ptr::null_mut();

const WND_CLASS_NAME: &str = "PurplWindow";

static mut WND_TITLE: String = String::new();
static mut WND_WIDTH: u32 = 0;
static mut WND_HEIGHT: u32 = 0;

static mut WND_RESIZED: bool = false;
static mut WND_FOCUSED: bool = false;
static mut WND_CLOSED: bool = false;

unsafe extern "system" fn wndproc(
    msgwnd: windef::HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if WND == std::ptr::null_mut() || msgwnd == WND {
        match msg {
            winuser::WM_SIZE => {
                let mut client_area: windef::RECT = std::mem::zeroed();
                let mut new_width: u32 = 0;
                let mut new_height: u32 = 0;

                winuser::GetClientRect(msgwnd, std::ptr::addr_of_mut!(client_area));
                new_width = (client_area.right - client_area.left) as u32;
                new_height = (client_area.bottom - client_area.top) as u32;

                if new_width != WND_WIDTH || new_height != WND_HEIGHT {
                    WND_RESIZED = true;
                    info!(
                        "Window resized from {}x{} to {}x{}",
                        WND_WIDTH, WND_HEIGHT, new_width, new_height
                    );
                }

                WND_WIDTH = new_width;
                WND_HEIGHT = new_height;
                0
            }
            winuser::WM_ACTIVATEAPP => {
                WND_FOCUSED = wparam != 0;
                info!(
                    "Window {}",
                    if WND_FOCUSED { "focused" } else { "unfocused" }
                );
                0
            }
            winuser::WM_DESTROY | winuser::WM_CLOSE => {
                info!("Window closed");
                WND_CLOSED = true;
                0
            }
            _ => winuser::DefWindowProcA(msgwnd, msg, wparam, lparam),
        }
    } else {
        winuser::DefWindowProcA(msgwnd, msg, wparam, lparam)
    }
}

unsafe fn register_wndclass() {
    let mut wnd_class: winuser::WNDCLASSEXA = std::mem::zeroed();
    let base_addr = libloaderapi::GetModuleHandleA(std::ptr::null_mut());

    debug!("Registering window class");

    wnd_class.cbSize = std::mem::size_of::<winuser::WNDCLASSEXA>() as u32;
    wnd_class.lpfnWndProc = Some(wndproc);
    wnd_class.hInstance = base_addr;
    wnd_class.hCursor = winuser::LoadCursorA(std::ptr::null_mut(), winuser::IDC_ARROW as *const i8);
    wnd_class.hIcon = winuser::LoadIconA(base_addr, IDI_ICON1 as *const i8);
    wnd_class.lpszClassName = WND_CLASS_NAME.as_ptr() as *const i8;
    if winuser::RegisterClassExA(std::ptr::addr_of_mut!(wnd_class)) == 0 {
        let err = errhandlingapi::GetLastError();
        panic!(
            "Failed to register window class: error 0x{:X} ({})",
            err, err
        );
    }

    debug!("Window class registered");
}

unsafe fn init_wnd() {
    let mut client_area: windef::RECT = std::mem::zeroed();
    let base_addr = libloaderapi::GetModuleHandleA(std::ptr::null_mut());

    client_area.left = 0;
    client_area.right = (winuser::GetSystemMetrics(winuser::SM_CXSCREEN) as f32 / 1.5) as i32;
    client_area.top = 0;
    client_area.bottom = (winuser::GetSystemMetrics(winuser::SM_CYSCREEN) as f32 / 1.5) as i32;
    winuser::AdjustWindowRect(
        std::ptr::addr_of_mut!(client_area),
        winuser::WS_OVERLAPPEDWINDOW,
        false as i32,
    );
    WND_WIDTH = (client_area.right - client_area.left) as u32;
    WND_HEIGHT = (client_area.bottom - client_area.top) as u32;

    WND_TITLE = std::format!("{} v{}", crate::GAME_NAME, crate::GAME_VERSION_STRING);
    debug!(
        "Creating {}x{} window titled {}",
        WND_WIDTH, WND_HEIGHT, WND_TITLE
    );

    WND = winuser::CreateWindowExA(
        0,
        WND_CLASS_NAME.as_ptr() as *const i8,
        WND_TITLE.as_ptr() as *const i8,
        winuser::WS_OVERLAPPEDWINDOW,
        winuser::CW_USEDEFAULT,
        winuser::CW_USEDEFAULT,
        WND_WIDTH as i32,
        WND_HEIGHT as i32,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        base_addr,
        std::ptr::null_mut(),
    );
    if WND == std::ptr::null_mut() {
        let err = errhandlingapi::GetLastError();
        panic!("Failed to create window: error 0x{:X} {}", err, err);
    }

    winuser::GetClientRect(WND, std::ptr::addr_of_mut!(client_area));
    WND_WIDTH = (client_area.right - client_area.left) as u32;
    WND_HEIGHT = (client_area.bottom - client_area.top) as u32;

    WND_RESIZED = false;
    WND_FOCUSED = true;
    WND_CLOSED = false;

    debug!(
        "Successfully created window with handle 0x{:X}",
        WND as usize
    );
}

pub unsafe fn init() {
    info!("Windows video initialization started");

    register_wndclass();
    init_wnd();

    debug!("Showing window");
    winuser::ShowWindow(WND, winuser::SW_SHOW);

    info!("Windows video initialization succeeded");
}

pub unsafe fn update() -> bool {
    let mut msg: winuser::MSG = std::mem::zeroed();

    while winuser::PeekMessageA(
        std::ptr::addr_of_mut!(msg),
        std::ptr::null_mut(),
        0,
        0,
        winuser::PM_REMOVE,
    ) != 0
    {
        winuser::TranslateMessage(std::ptr::addr_of_mut!(msg));
        winuser::DispatchMessageA(std::ptr::addr_of_mut!(msg));
    }

    !WND_CLOSED
}

pub fn shutdown() {
    info!("Windows video shutdown started");

    debug!("Destroying window");
    unsafe {
        winuser::DestroyWindow(WND);
    }

    info!("Windows video shutdown succeeded");
}
