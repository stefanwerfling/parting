use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::snap::{self, SnapConfig, SnapZone};
use crate::xrandr::{self, Monitor, Output, SplitSpec};
use eframe::egui;

pub struct PartingApp {
    outputs: Vec<Output>,
    monitors: Vec<Monitor>,
    selected_output: Option<String>,
    split_count: usize,
    split_ratios: Vec<f32>,
    status: String,
    last_error: Option<String>,

    show_dividers: bool,
    divider_width_px: f32,
    divider_rgb: [u8; 3],
    divider_opacity: u8,

    snap_enabled_flag: Arc<AtomicBool>,
    snap_config: Arc<Mutex<SnapConfig>>,
    snap_trigger_radius: i32,
}

#[derive(Debug, Clone, Copy)]
struct DividerRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl PartingApp {
    pub fn new() -> Self {
        let snap_enabled_flag = Arc::new(AtomicBool::new(false));
        let snap_config = Arc::new(Mutex::new(SnapConfig::default()));
        let snap_stop = Arc::new(AtomicBool::new(false));
        snap::spawn_snap_daemon(snap_config.clone(), snap_enabled_flag.clone(), snap_stop);

        let mut app = Self {
            outputs: Vec::new(),
            monitors: Vec::new(),
            selected_output: None,
            split_count: 2,
            split_ratios: vec![0.5, 0.5],
            status: String::new(),
            last_error: None,
            show_dividers: false,
            divider_width_px: 3.0,
            divider_rgb: [0, 220, 220],
            divider_opacity: 220,
            snap_enabled_flag,
            snap_config,
            snap_trigger_radius: 40,
        };
        app.refresh();
        app.push_snap_zones();
        app
    }

    fn push_snap_zones(&self) {
        let zones: Vec<SnapZone> = self
            .monitors
            .iter()
            .filter(|m| m.is_virtual)
            .map(|m| SnapZone {
                x: m.x,
                y: m.y,
                width: m.width_px,
                height: m.height_px,
            })
            .collect();
        if let Ok(mut cfg) = self.snap_config.lock() {
            cfg.zones = zones;
            cfg.trigger_radius = self.snap_trigger_radius;
        }
    }

    fn refresh(&mut self) {
        match xrandr::list_outputs() {
            Ok(o) => self.outputs = o,
            Err(e) => self.last_error = Some(format!("list_outputs: {e}")),
        }
        match xrandr::list_monitors() {
            Ok(m) => self.monitors = m,
            Err(e) => self.last_error = Some(format!("list_monitors: {e}")),
        }
        let still_valid = self
            .selected_output
            .as_ref()
            .and_then(|n| self.outputs.iter().find(|o| &o.name == n))
            .map(|o| o.connected)
            .unwrap_or(false);
        if !still_valid {
            self.selected_output = self
                .outputs
                .iter()
                .find(|o| o.connected)
                .map(|o| o.name.clone());
        }
        self.push_snap_zones();
    }

    fn selected(&self) -> Option<&Output> {
        self.selected_output
            .as_ref()
            .and_then(|n| self.outputs.iter().find(|o| &o.name == n))
    }

    fn build_splits(&self) -> Option<Vec<SplitSpec>> {
        let out = self.selected()?;
        if !out.connected {
            return None;
        }
        let sum: f32 = self.split_ratios.iter().sum();
        if sum <= 0.0 {
            return None;
        }

        let mut acc = 0.0f32;
        let mut result = Vec::with_capacity(self.split_ratios.len());
        for (i, r) in self.split_ratios.iter().enumerate() {
            let start_ratio = acc / sum;
            let end_ratio = (acc + r) / sum;
            acc += r;

            let x_start = (out.width_px as f32 * start_ratio).round() as u32;
            let x_end = (out.width_px as f32 * end_ratio).round() as u32;
            let width_px = x_end.saturating_sub(x_start);

            let mm_start = (out.width_mm as f32 * start_ratio).round() as u32;
            let mm_end = (out.width_mm as f32 * end_ratio).round() as u32;
            let width_mm = mm_end.saturating_sub(mm_start);

            result.push(SplitSpec {
                name: format!("{}~{}", out.name, part_label(i, self.split_count)),
                width_px,
                height_px: out.height_px,
                width_mm,
                height_mm: out.height_mm,
                x: out.x + x_start as i32,
                y: out.y,
            });
        }
        Some(result)
    }

    fn divider_color(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            self.divider_rgb[0],
            self.divider_rgb[1],
            self.divider_rgb[2],
            self.divider_opacity,
        )
    }

    /// Errechnet die vertikalen Trennlinien-Rechtecke zwischen benachbarten
    /// virtuellen Monitoren (gleiche Zeile, direkt anschließend).
    fn compute_dividers(&self) -> Vec<DividerRect> {
        let mut vs: Vec<&Monitor> = self.monitors.iter().filter(|m| m.is_virtual).collect();
        vs.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)));

        let w = self.divider_width_px.max(1.0).round() as u32;
        let w_i = w as i32;
        let mut dividers = Vec::new();
        for pair in vs.windows(2) {
            let a = pair[0];
            let b = pair[1];
            if a.y == b.y && a.height_px == b.height_px {
                let a_right = a.x + a.width_px as i32;
                if (b.x - a_right).abs() <= 4 {
                    dividers.push(DividerRect {
                        x: a_right - w_i / 2,
                        y: a.y,
                        width: w,
                        height: a.height_px,
                    });
                }
            }
        }
        dividers
    }
}

fn part_label(idx: usize, count: usize) -> &'static str {
    match (idx, count) {
        (0, 2) => "L",
        (1, 2) => "R",
        (0, 3) => "L",
        (1, 3) => "M",
        (2, 3) => "R",
        (0, 4) => "A",
        (1, 4) => "B",
        (2, 4) => "C",
        (3, 4) => "D",
        _ => "X",
    }
}

impl eframe::App for PartingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Parting");
                ui.label("virtual monitor splitter (X11 / xrandr)");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Aktualisieren").clicked() {
                        self.refresh();
                    }
                });
            });
        });

        egui::SidePanel::left("outputs")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Physische Ausgänge");
                    let outputs = self.outputs.clone();
                    for o in &outputs {
                        let text = if o.connected {
                            format!("{}  ·  {}×{}", o.name, o.width_px, o.height_px)
                        } else {
                            format!("{}  ·  disconnected", o.name)
                        };
                        let selected = self.selected_output.as_deref() == Some(o.name.as_str());
                        let resp = ui.add_enabled(
                            o.connected,
                            egui::SelectableLabel::new(selected, text),
                        );
                        if resp.clicked() {
                            self.selected_output = Some(o.name.clone());
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.heading("Aktive Monitore");
                    for m in &self.monitors {
                        let icon = if m.is_virtual { "◨" } else { "▪" };
                        let primary = if m.is_primary { " ★" } else { "" };
                        ui.label(format!(
                            "{} {}{}  ·  {}×{} @ ({},{})",
                            icon, m.name, primary, m.width_px, m.height_px, m.x, m.y
                        ));
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            let out = match self.selected().cloned() {
                Some(o) => o,
                None => {
                    ui.label("Kein aktiver Ausgang ausgewählt.");
                    return;
                }
            };

            ui.heading(format!("Split für: {}", out.name));
            ui.label(format!(
                "Native: {}×{} px · {}×{} mm · Position ({}, {})",
                out.width_px, out.height_px, out.width_mm, out.height_mm, out.x, out.y
            ));

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Anzahl Splits:");
                for n in [2usize, 3, 4] {
                    if ui
                        .selectable_label(self.split_count == n, n.to_string())
                        .clicked()
                        && self.split_count != n
                    {
                        self.split_count = n;
                        self.split_ratios = vec![1.0 / n as f32; n];
                    }
                }
                if ui.button("gleich verteilen").clicked() {
                    let n = self.split_count;
                    self.split_ratios = vec![1.0 / n as f32; n];
                }
            });

            ui.add_space(6.0);
            ui.label("Verhältnisse (werden auf 1 normiert):");
            for (i, r) in self.split_ratios.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Teil {}", i + 1));
                    ui.add(
                        egui::Slider::new(r, 0.05..=1.0)
                            .fixed_decimals(2)
                            .step_by(0.01),
                    );
                });
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label("Vorschau:");
            draw_preview(ui, &out, &self.split_ratios, self.split_count);

            ui.add_space(10.0);
            ui.separator();

            let splits = self.build_splits();
            if let Some(ref sp) = splits {
                ui.collapsing("Details der geplanten Splits", |ui| {
                    for s in sp {
                        ui.label(format!(
                            "· {}  →  {}×{} px  @ ({},{})  ·  {}×{} mm",
                            s.name, s.width_px, s.height_px, s.x, s.y, s.width_mm, s.height_mm
                        ));
                    }
                });
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let apply = ui.button("✓ Split anwenden");
                let reset = ui.button("↺ Alle virtuellen Monitore entfernen");

                if apply.clicked() {
                    if let Some(sp) = splits {
                        match xrandr::apply_split(&out.name, &sp) {
                            Ok(_) => {
                                self.status = format!("{} Splits für {} angewendet.", sp.len(), out.name);
                                self.last_error = None;
                            }
                            Err(e) => self.last_error = Some(format!("apply_split: {e}")),
                        }
                        self.refresh();
                    }
                }
                if reset.clicked() {
                    match xrandr::remove_all_virtual() {
                        Ok(n) => {
                            self.status = format!("{n} virtuelle Monitore entfernt.");
                            self.last_error = None;
                        }
                        Err(e) => self.last_error = Some(format!("remove_all: {e}")),
                    }
                    self.refresh();
                }
            });

            if !self.status.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), &self.status);
            }
            if let Some(e) = &self.last_error {
                ui.colored_label(egui::Color32::from_rgb(220, 100, 100), format!("Fehler: {e}"));
            }

            ui.add_space(14.0);
            ui.separator();
            ui.heading("Trennlinien-Overlay");
            ui.label(
                "Zeigt an den Grenzen der virtuellen Monitore eine dünne farbige Linie \
                 (transparent, immer im Vordergrund, klick-durchlässig). Rein visuell — \
                 verändert nichts an den Splits selbst.",
            );

            ui.checkbox(&mut self.show_dividers, "Trennlinien anzeigen");

            ui.add_enabled_ui(self.show_dividers, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Breite (px):");
                    ui.add(egui::Slider::new(&mut self.divider_width_px, 1.0..=20.0).integer());
                });
                ui.horizontal(|ui| {
                    ui.label("Deckkraft:");
                    ui.add(egui::Slider::new(&mut self.divider_opacity, 20..=255));
                });
                ui.horizontal(|ui| {
                    ui.label("Farbe:");
                    ui.color_edit_button_srgb(&mut self.divider_rgb);
                    if ui.small_button("Cyan").clicked() { self.divider_rgb = [0, 220, 220]; }
                    if ui.small_button("Rot").clicked() { self.divider_rgb = [230, 60, 60]; }
                    if ui.small_button("Weiß").clicked() { self.divider_rgb = [240, 240, 240]; }
                    if ui.small_button("Gelb").clicked() { self.divider_rgb = [255, 210, 50]; }
                });
                let n = self.compute_dividers().len();
                ui.label(format!("Aktive Grenzen: {n}"));
            });

            ui.add_space(14.0);
            ui.separator();
            ui.heading("Fenster-Andock");
            ui.label(
                "Beim Ziehen und Loslassen eines Fensters nahe einer virtuellen \
                 Monitor-Grenze wird es auf die entsprechende Hälfte gesnapped.",
            );
            let mut snap_on = self.snap_enabled_flag.load(Ordering::Relaxed);
            if ui.checkbox(&mut snap_on, "Fenster-Andock aktiv").changed() {
                self.snap_enabled_flag.store(snap_on, Ordering::Relaxed);
                self.push_snap_zones();
            }
            ui.add_enabled_ui(snap_on, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Fangradius (px):");
                    if ui
                        .add(egui::Slider::new(&mut self.snap_trigger_radius, 5..=200))
                        .changed()
                    {
                        self.push_snap_zones();
                    }
                });
                let zones = self
                    .snap_config
                    .lock()
                    .map(|c| c.zones.len())
                    .unwrap_or(0);
                ui.label(format!("Aktive Snap-Zonen: {zones}"));
                ui.label(
                    "Hinweis: das aktive Fenster ist entscheidend — vor dem Ziehen \
                     kurz auf den Titel klicken, damit es fokussiert ist.",
                );
            });
            });
        });

        // Overlay-Viewports rendern
        if self.show_dividers {
            let color = self.divider_color();
            let dividers = self.compute_dividers();
            for (idx, d) in dividers.iter().enumerate() {
                let x = d.x as f32;
                let y = d.y as f32;
                let w = d.width as f32;
                let h = d.height as f32;
                let vp_id = egui::ViewportId::from_hash_of(format!("parting-divider-{idx}"));
                let title = format!("parting-divider-{idx}");
                ctx.show_viewport_deferred(
                    vp_id,
                    egui::ViewportBuilder::default()
                        .with_title(title)
                        .with_position([x, y])
                        .with_inner_size([w, h])
                        .with_decorations(false)
                        .with_transparent(true)
                        .with_always_on_top()
                        .with_mouse_passthrough(true)
                        .with_resizable(false),
                    move |ctx, _class| {
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(color))
                            .show(ctx, |_ui| {});
                    },
                );
            }
        }
    }
}

fn draw_preview(ui: &mut egui::Ui, out: &Output, ratios: &[f32], count: usize) {
    let avail = ui.available_width().min(720.0).max(200.0);
    let aspect = if out.height_px > 0 {
        out.width_px as f32 / out.height_px as f32
    } else {
        16.0 / 9.0
    };
    let h = (avail / aspect).clamp(80.0, 240.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail, h), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(28));
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)));

    let total: f32 = ratios.iter().sum();
    if total <= 0.0 {
        return;
    }
    let colors = [
        egui::Color32::from_rgb(70, 130, 200),
        egui::Color32::from_rgb(200, 130, 70),
        egui::Color32::from_rgb(120, 180, 100),
        egui::Color32::from_rgb(180, 120, 200),
    ];

    let mut acc = 0.0f32;
    for (i, r) in ratios.iter().enumerate() {
        let start = acc / total;
        let end = (acc + r) / total;
        acc += r;
        let x0 = rect.left() + rect.width() * start + 2.0;
        let x1 = rect.left() + rect.width() * end - 2.0;
        let seg = egui::Rect::from_min_max(
            egui::pos2(x0, rect.top() + 2.0),
            egui::pos2(x1.max(x0 + 1.0), rect.bottom() - 2.0),
        );
        painter.rect_filled(seg, 3.0, colors[i % colors.len()]);
        let label = format!("{}~{}", out.name, part_label(i, count));
        painter.text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
        let width_px = ((end - start) * out.width_px as f32).round() as u32;
        painter.text(
            egui::pos2(seg.center().x, seg.center().y + 16.0),
            egui::Align2::CENTER_CENTER,
            format!("{}×{}", width_px, out.height_px),
            egui::FontId::proportional(11.0),
            egui::Color32::from_gray(240),
        );
    }
}