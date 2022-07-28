use std::sync::Arc;

use anyhow::Result;
use x11::keysym;
use x11::xlib;
use xcb;
use xcb::x;

const MAP_NOTIFY_EVENT: u8 = 19;
const ENTER_NOTIFY_EVENT: u8 = 7;
const DESTROY_NOTIFY_EVENT: u8 = 17;
const BUTTON_PRESS_EVENT: u8 = 4;
const BUTTON_RELEASE_EVENT: u8 = 5;
const KEY_PRESS_EVENT: u8 = 2;
const MAP_REQUEST_EVENT: u8 = 20;
const FOCUS_IN_EVENT: u8 = 9;
const FOCUS_OUT_EVENT: u8 = 10;
const MAPPING_NOTIFY_EVENT: u8 = 34;

// #[derive(Debug)]
struct Key {
    mod_key: x::KeyButMask,
    keysym: xcb::x::Keysym,
    func: Arc<dyn Fn()>,
}

type EventHandler = Arc<dyn Fn(&xcb::Connection, &x::Event)>;

struct Handler {
    request: u8,
    handler: EventHandler,
}

struct WM {
    keys: Vec<Key>,
    handlers: Vec<Handler>,
}

fn on_motion_notify(conn: &xcb::Connection, ev: &x::Event) {
    println!("On map notify...");
}

fn on_keypress_event(wm: &WM, conn: &xcb::Connection, display: *mut xlib::Display, ev: &x::Event) {
    println!("On Key Press event...");
    if let x::Event::KeyPress(e) = ev {
        let keysym = get_keysym(display, e.detail());
        for key in &wm.keys {
            if key.keysym == keysym && key.mod_key == e.state() {
                println!("key found...");
                (*key.func)();
            }
        }
    }
}

fn on_mapping_notify(conn: &xcb::Connection, ev: &x::Event) {
    println!("On Mapping Notify...");
}

fn on_map_request_event(conn: &xcb::Connection, ev: &x::Event) {
    println!("On Map Request...");
}

fn event_handler(
    wm: &WM,
    conn: &xcb::Connection,
    display: *mut xlib::Display,
) -> anyhow::Result<()> {
    conn.has_error()?;

    let e = conn.wait_for_event()?;
    println!("{:?}", e);
    match e {
        xcb::Event::X(ref s @ x::Event::KeyPress(ref ev)) => match ev.response_type() {
            MAP_NOTIFY_EVENT => on_motion_notify(conn, &s),
            KEY_PRESS_EVENT => on_keypress_event(wm, conn, display, &s),
            MAPPING_NOTIFY_EVENT => on_mapping_notify(conn, &s),
            MAP_REQUEST_EVENT => on_map_request_event(conn, &s),
            _ => {
                println!("ev: {:?}", ev)
            }
        },
        _ => {
            println!("e: {:?}", e)
        }
    }

    Ok(())
}

fn get_keycode(display: *mut xlib::Display, keysym: xlib::KeySym) -> u8 {
    unsafe { xlib::XKeysymToKeycode(display, keysym) }
}

fn get_keysym(display: *mut xlib::Display, keycode: u8) -> xcb::x::Keysym {
    unsafe { xlib::XkbKeycodeToKeysym(display, keycode, 0 as i32, 0 as i32) as u32 }
}

fn get_display() -> *mut xlib::Display {
    unsafe { xlib::XOpenDisplay(std::ptr::null()) }
}

fn get_xcb_connection(display: *mut xlib::Display) -> xcb::Connection {
    unsafe { xcb::Connection::from_xlib_display(display) }
}

fn main() -> Result<()> {
    println!("Samurai WM");

    let display = get_display();
    let handlers = vec![Handler {
        request: MAP_NOTIFY_EVENT,
        handler: Arc::new(on_motion_notify),
    }];

    let mut wm = WM {
        keys: vec![Key {
            mod_key: x::KeyButMask::MOD4,
            keysym: keysym::XK_t,
            func: Arc::new(|| println!("launch terminal...")),
        }],
        handlers,
    };

    // let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let conn = get_xcb_connection(display);
    let setup = conn.get_setup();
    let screen = setup.roots().nth(0).unwrap();

    conn.send_request(&x::ChangeWindowAttributes {
        window: screen.root(),
        value_list: &[x::Cw::EventMask(
            x::EventMask::SUBSTRUCTURE_REDIRECT
                | x::EventMask::STRUCTURE_NOTIFY
                | x::EventMask::SUBSTRUCTURE_NOTIFY
                | x::EventMask::PROPERTY_CHANGE
                // | x::EventMask::KEY_PRESS
                // | x::EventMask::KEY_RELEASE
                | x::EventMask::POINTER_MOTION,
        )],
    });

    conn.send_request(&x::UngrabKey {
        key: x::GRAB_ANY,
        grab_window: screen.root(),
        modifiers: x::ModMask::ANY,
    });

    for key in &wm.keys {
        println!("Keysym: {}", key.keysym);
        let keycode = get_keycode(display, key.keysym.into());
        println!("Keycode: {}", keycode);

        conn.send_request(&x::GrabKey {
            owner_events: true,
            grab_window: screen.root(),
            key: keycode,
            modifiers: x::ModMask::from_bits(key.mod_key.bits()).unwrap(),
            keyboard_mode: x::GrabMode::Async,
            pointer_mode: x::GrabMode::Async,
        });
    }

    conn.flush()?;

    conn.send_request(&x::GrabButton {
        owner_events: false,
        grab_window: screen.root(),
        event_mask: x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE,
        pointer_mode: x::GrabMode::Async,
        keyboard_mode: x::GrabMode::Async,
        confine_to: screen.root(),
        cursor: x::CURSOR_NONE,
        button: x::ButtonIndex::N1,
        modifiers: x::ModMask::N4,
    });

    conn.send_request(&x::GrabButton {
        owner_events: false,
        grab_window: screen.root(),
        event_mask: x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE,
        pointer_mode: x::GrabMode::Async,
        keyboard_mode: x::GrabMode::Async,
        confine_to: screen.root(),
        cursor: x::CURSOR_NONE,
        button: x::ButtonIndex::N3,
        modifiers: x::ModMask::N4,
    });

    conn.flush()?;

    loop {
        if let Err(_ret) = event_handler(&wm, &conn, display) {
            break;
        }
    }

    Ok(())
}
