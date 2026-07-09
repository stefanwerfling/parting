use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConnectionExt, EventMask, KeyButMask, Window,
};

#[derive(Debug, Clone, Copy)]
pub struct SnapZone {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct SnapConfig {
    pub zones: Vec<SnapZone>,
    /// Wie nah der Cursor beim Loslassen an einer Zonen-Innenkante sein muss,
    /// damit gesnapped wird.
    pub trigger_radius: i32,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self { zones: Vec::new(), trigger_radius: 40 }
    }
}

/// Startet einen Hintergrund-Thread, der Maus-Bewegungen pollt und bei
/// Drag-Ende in der Nähe einer Zonen-Grenze das aktive Fenster snapped.
pub fn spawn_snap_daemon(
    config: Arc<Mutex<SnapConfig>>,
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[snap] x11rb connect failed: {e}");
                return;
            }
        };
        let root = conn.setup().roots[screen_num].root;

        let intern = |name: &[u8]| -> Option<u32> {
            conn.intern_atom(false, name).ok()?.reply().ok().map(|r| r.atom)
        };
        let net_active = match intern(b"_NET_ACTIVE_WINDOW") { Some(a) => a, None => return };
        let net_moveresize = match intern(b"_NET_MOVERESIZE_WINDOW") { Some(a) => a, None => return };
        let net_wm_state = match intern(b"_NET_WM_STATE") { Some(a) => a, None => return };
        let max_horz = match intern(b"_NET_WM_STATE_MAXIMIZED_HORZ") { Some(a) => a, None => return };
        let max_vert = match intern(b"_NET_WM_STATE_MAXIMIZED_VERT") { Some(a) => a, None => return };

        let mut drag_active = false;
        let mut drag_start = (0i32, 0i32);
        let mut drag_window: Option<Window> = None;

        while !stop.load(Ordering::Relaxed) {
            if !enabled.load(Ordering::Relaxed) {
                drag_active = false;
                drag_window = None;
                thread::sleep(Duration::from_millis(200));
                continue;
            }

            let ptr = match conn.query_pointer(root).ok().and_then(|c| c.reply().ok()) {
                Some(p) => p,
                None => {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
            };
            let btn1_down = (ptr.mask & KeyButMask::BUTTON1) == KeyButMask::BUTTON1;
            let cur_x = ptr.root_x as i32;
            let cur_y = ptr.root_y as i32;

            if btn1_down && !drag_active {
                if let Some(w) = get_active_window(&conn, root, net_active) {
                    drag_active = true;
                    drag_start = (cur_x, cur_y);
                    drag_window = Some(w);
                }
            } else if !btn1_down && drag_active {
                let dx = (cur_x - drag_start.0).abs();
                let dy = (cur_y - drag_start.1).abs();
                let moved = dx > 8 || dy > 8;
                if moved {
                    let (zones, radius) = {
                        let cfg = config.lock().unwrap();
                        (cfg.zones.clone(), cfg.trigger_radius)
                    };
                    if let Some(zone) = pick_zone(&zones, cur_x, cur_y, radius) {
                        if let Some(w) = drag_window {
                            let _ = unmaximize(&conn, root, w, net_wm_state, max_horz, max_vert);
                            let _ = moveresize(&conn, root, w, net_moveresize, &zone);
                            let _ = conn.flush();
                        }
                    }
                }
                drag_active = false;
                drag_window = None;
            }

            thread::sleep(Duration::from_millis(30));
        }
    });
}

fn get_active_window<C: Connection>(conn: &C, root: Window, net_active: u32) -> Option<Window> {
    let reply = conn
        .get_property(false, root, net_active, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let w = reply.value32()?.next()?;
    if w == 0 { None } else { Some(w) }
}

/// Wählt eine Zone aus, wenn der Cursor darin liegt und nahe genug an einer
/// _inneren_ Kante (nur relevant, wenn mehrere Zonen nebeneinander liegen).
fn pick_zone(zones: &[SnapZone], x: i32, y: i32, radius: i32) -> Option<SnapZone> {
    for z in zones {
        let inside = x >= z.x
            && x < z.x + z.width as i32
            && y >= z.y
            && y < z.y + z.height as i32;
        if !inside {
            continue;
        }
        let has_left_neighbor = zones.iter().any(|o| {
            o.y == z.y && o.height == z.height && (o.x + o.width as i32 - z.x).abs() <= 4
        });
        let has_right_neighbor = zones.iter().any(|o| {
            o.y == z.y && o.height == z.height && (z.x + z.width as i32 - o.x).abs() <= 4
        });
        let dist_left = x - z.x;
        let dist_right = z.x + z.width as i32 - x;
        let near_left = has_left_neighbor && dist_left < radius;
        let near_right = has_right_neighbor && dist_right < radius;
        if near_left || near_right {
            return Some(*z);
        }
    }
    None
}

fn unmaximize<C: Connection>(
    conn: &C,
    root: Window,
    w: Window,
    net_wm_state: u32,
    max_horz: u32,
    max_vert: u32,
) -> anyhow::Result<()> {
    // _NET_WM_STATE_REMOVE = 0
    let event = ClientMessageEvent::new(32, w, net_wm_state, [0u32, max_horz, max_vert, 1u32, 0u32]);
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
        event,
    )?;
    Ok(())
}

fn moveresize<C: Connection>(
    conn: &C,
    root: Window,
    w: Window,
    net_moveresize: u32,
    zone: &SnapZone,
) -> anyhow::Result<()> {
    // _NET_MOVERESIZE_WINDOW flags:
    //   bits 0-7:  gravity (0 = default gravity from WM hints)
    //   bit 8:  x present
    //   bit 9:  y present
    //   bit 10: width present
    //   bit 11: height present
    //   bits 12-15: source indication (1 = normal application)
    let flags: u32 = (1 << 8) | (1 << 9) | (1 << 10) | (1 << 11) | (1 << 12);
    let event = ClientMessageEvent::new(
        32,
        w,
        net_moveresize,
        [flags, zone.x as u32, zone.y as u32, zone.width, zone.height],
    );
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
        event,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_zone_only_snaps_near_shared_edge() {
        let left = SnapZone { x: 0, y: 0, width: 100, height: 100 };
        let right = SnapZone { x: 100, y: 0, width: 100, height: 100 };
        let zones = vec![left, right];

        // Cursor in left, weit weg von der geteilten Kante → kein Snap
        assert!(pick_zone(&zones, 10, 50, 20).is_none());
        // Cursor in left, nah an geteilter Kante rechts → snap zu left
        let picked = pick_zone(&zones, 90, 50, 20).unwrap();
        assert_eq!(picked.x, 0);
        // Cursor in right, nah an geteilter Kante links → snap zu right
        let picked = pick_zone(&zones, 105, 50, 20).unwrap();
        assert_eq!(picked.x, 100);
        // Cursor komplett außerhalb → None
        assert!(pick_zone(&zones, 500, 500, 20).is_none());
    }

    #[test]
    fn pick_zone_ignores_outer_edges() {
        // Alleinstehende Zone: kein Nachbar → nirgends soll gesnapped werden
        let alone = SnapZone { x: 0, y: 0, width: 100, height: 100 };
        let zones = vec![alone];
        assert!(pick_zone(&zones, 5, 50, 20).is_none());
        assert!(pick_zone(&zones, 95, 50, 20).is_none());
    }
}