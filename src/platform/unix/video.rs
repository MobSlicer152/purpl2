use log::{debug, info};
use std::mem;
use xcb::x;

static mut XCB_CONNECTION: Option<xcb::Connection> = None;

static mut XCB_WND: Option<x::Window> = None;

static mut WND_TITLE: String = String::new();
static mut WND_WIDTH: u32 = 0;
static mut WND_HEIGHT: u32 = 0;

static mut WND_RESIZED: bool = false;
static mut WND_FOCUSED: bool = false;
static mut WND_CLOSED: bool = false;

unsafe fn get_xcb_atom(name: &str) -> x::Atom {
    let conn = XCB_CONNECTION.as_ref().unwrap();
    let reply = conn.send_request(&x::InternAtom {
        only_if_exists: true,
        name: name.as_bytes(),
    });
    conn.wait_for_reply(reply).unwrap().atom()
}

pub unsafe fn init() {
    info!("XCB video initialization started");

    let conn = xcb::Connection::connect(None).unwrap();
    XCB_CONNECTION = Some(conn.0);
    let screen_num = conn.1;
    let conn = XCB_CONNECTION.as_ref().unwrap();

    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    WND_HEIGHT = (screen.height_in_pixels() as f32 / 1.5) as u32;
    WND_WIDTH = (WND_HEIGHT as f32 * 1.777777777777777) as u32;

    let wnd = conn.generate_id();
    XCB_WND = Some(wnd);
    let wnd = XCB_WND.as_ref().unwrap();
    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: *wnd,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: WND_WIDTH as u16,
        height: WND_HEIGHT as u16,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: screen.root_visual(),
        value_list: &[
            x::Cw::BackPixel(screen.black_pixel()),
            x::Cw::EventMask(x::EventMask::FOCUS_CHANGE | x::EventMask::STRUCTURE_NOTIFY),
        ],
    });
    if conn.check_request(cookie).is_err() {
        panic!("Failed to create window");
    }

    let cookie = conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: *wnd,
        property: x::ATOM_WM_NAME,
        r#type: x::ATOM_STRING,
        data: WND_TITLE.as_bytes(),
    });
    if conn.check_request(cookie).is_err() {
        panic!("Failed to set window title to {}", WND_TITLE);
    }

    let protocols = get_xcb_atom("WM_PROTOCOLS");
    let cookie = conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: *wnd,
        property: protocols,
        r#type: x::ATOM_ATOM,
        data: &[get_xcb_atom("WM_DELETE_WINDOW")],
    });

    conn.send_request(&x::MapWindow { window: *wnd });

    if conn.flush().is_err() {
        panic!("Failed to flush XCB connection");
    }

    info!("XCB video initialization succeeded");
}

pub unsafe fn update() -> bool {
    let conn = XCB_CONNECTION.as_ref().unwrap();
    if let Ok(Some(xcb::Event::X(event))) = conn.poll_for_event() {
        match event {
            x::Event::ConfigureNotify(ev) => {
                let new_width = ev.width() as u32;
                let new_height = ev.height() as u32;

                if new_width != WND_WIDTH || new_height != WND_HEIGHT {
                    WND_RESIZED = true;
                    info!(
                        "Window resized from {}x{} to {}x{}",
                        WND_WIDTH, WND_HEIGHT, new_width, new_height
                    );
                    WND_WIDTH = new_width;
                    WND_HEIGHT = new_height;
                }
            }
            x::Event::FocusIn(_) => {
                info!("Window focused");
                WND_FOCUSED = true;
            }
            x::Event::FocusOut(_) => {
                info!("Window unfocused");
                WND_FOCUSED = false;
            }
            x::Event::ClientMessage(ev) => {
                if let x::ClientMessageData::Data32(atom) = ev.data() {
                    let delete_atom = get_xcb_atom("WM_DELETE_WINDOW");
                    let atom = mem::transmute::<u32, x::Atom>(atom[0]);
                    WND_CLOSED = atom == delete_atom;
                }
            }
            _ => {}
        }
    }

    !WND_CLOSED
}

pub unsafe fn shutdown() {
    let conn = XCB_CONNECTION.as_ref().unwrap();

    info!("XCB video shutdown started");

    debug!("Destroying window");
    conn.send_request(&x::DestroyWindow {
        window: XCB_WND.unwrap(),
    });

    info!("XCB video shutdown succeeded");
}

pub unsafe fn set_size(mut width: &u32, mut height: &u32) {
    width = &WND_WIDTH;
    height = &WND_HEIGHT;
}

pub unsafe fn resized() -> bool {
    let ret = WND_RESIZED;
    WND_RESIZED = false;
    ret
}

pub unsafe fn focused() -> bool {
    WND_FOCUSED
}
