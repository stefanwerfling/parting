use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConnectionExt, EventMask, Window,
};

/// Startet einen Hintergrund-Thread, der periodisch alle Toplevel-Fenster mit
/// dem angegebenen Titelpräfix in _NET_WM_STATE_SKIP_TASKBAR/_SKIP_PAGER versetzt.
///
/// Wir müssen das machen, weil winit auf X11 kein set_skip_taskbar hat und
/// eframes with_taskbar(false) auf Linux effektiv wirkungslos ist.
pub fn spawn_taskbar_hider(name_prefix: String, stop: Arc<AtomicBool>) {
    thread::spawn(move || {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[x11_helper] x11rb connect failed: {e}");
                return;
            }
        };
        let root = conn.setup().roots[screen_num].root;

        let atom = |name: &[u8]| -> Option<u32> {
            conn.intern_atom(false, name)
                .ok()?
                .reply()
                .ok()
                .map(|r| r.atom)
        };

        let net_wm_state = match atom(b"_NET_WM_STATE") {
            Some(a) => a,
            None => return,
        };
        let skip_taskbar = match atom(b"_NET_WM_STATE_SKIP_TASKBAR") {
            Some(a) => a,
            None => return,
        };
        let skip_pager = match atom(b"_NET_WM_STATE_SKIP_PAGER") {
            Some(a) => a,
            None => return,
        };
        let net_wm_name = match atom(b"_NET_WM_NAME") {
            Some(a) => a,
            None => return,
        };
        let utf8_string = match atom(b"UTF8_STRING") {
            Some(a) => a,
            None => return,
        };
        let net_client_list = match atom(b"_NET_CLIENT_LIST") {
            Some(a) => a,
            None => return,
        };

        while !stop.load(Ordering::Relaxed) {
            let windows = list_client_windows(&conn, root, net_client_list).unwrap_or_default();
            for w in windows {
                let title = read_wm_name(&conn, w, net_wm_name, utf8_string).unwrap_or_default();
                if title.starts_with(&name_prefix) {
                    let _ = send_skip_taskbar(
                        &conn,
                        root,
                        w,
                        net_wm_state,
                        skip_taskbar,
                        skip_pager,
                    );
                }
            }
            let _ = conn.flush();
            thread::sleep(Duration::from_millis(500));
        }
    });
}

fn list_client_windows<C: Connection>(
    conn: &C,
    root: Window,
    net_client_list: u32,
) -> anyhow::Result<Vec<Window>> {
    let reply = conn
        .get_property(false, root, net_client_list, AtomEnum::WINDOW, 0, 4096)?
        .reply()?;
    Ok(reply.value32().map(|it| it.collect()).unwrap_or_default())
}

fn read_wm_name<C: Connection>(
    conn: &C,
    w: Window,
    net_wm_name: u32,
    utf8_string: u32,
) -> anyhow::Result<String> {
    let reply = conn
        .get_property(false, w, net_wm_name, utf8_string, 0, 256)?
        .reply()?;
    Ok(String::from_utf8_lossy(&reply.value).into_owned())
}

fn send_skip_taskbar<C: Connection>(
    conn: &C,
    root: Window,
    target: Window,
    net_wm_state: u32,
    skip_taskbar: u32,
    skip_pager: u32,
) -> anyhow::Result<()> {
    // _NET_WM_STATE ClientMessage:
    //   data.l[0] = action: 1 = _NET_WM_STATE_ADD
    //   data.l[1] = first atom to add
    //   data.l[2] = second atom (or 0)
    //   data.l[3] = source indication: 1 = normal application
    //   data.l[4] = 0
    let event = ClientMessageEvent::new(
        32,
        target,
        net_wm_state,
        [1u32, skip_taskbar, skip_pager, 1u32, 0u32],
    );
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
        event,
    )?;
    Ok(())
}