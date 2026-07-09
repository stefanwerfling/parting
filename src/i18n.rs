#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    De,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::De, Lang::En];

    pub fn flag_label(self) -> &'static str {
        match self {
            Lang::De => "🇩🇪 Deutsch",
            Lang::En => "🇬🇧 English",
        }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::De => &DE,
            Lang::En => &EN,
        }
    }
}

pub struct Strings {
    pub app_subtitle: &'static str,
    pub refresh: &'static str,
    pub language: &'static str,

    pub physical_outputs: &'static str,
    pub disconnected: &'static str,
    pub active_monitors: &'static str,

    pub no_output_selected: &'static str,
    pub split_for: &'static str,
    pub native_prefix: &'static str,
    pub position: &'static str,

    pub split_count: &'static str,
    pub distribute_evenly: &'static str,
    pub ratios_normalized: &'static str,
    pub part: &'static str,
    pub preview: &'static str,
    pub details: &'static str,

    pub apply_split: &'static str,
    pub reset_all: &'static str,
    pub status_applied: &'static str,
    pub status_removed_prefix: &'static str,
    pub status_removed_suffix: &'static str,
    pub error_prefix: &'static str,

    pub dividers_heading: &'static str,
    pub dividers_description: &'static str,
    pub show_dividers: &'static str,
    pub width_px: &'static str,
    pub opacity: &'static str,
    pub color: &'static str,
    pub color_cyan: &'static str,
    pub color_red: &'static str,
    pub color_white: &'static str,
    pub color_yellow: &'static str,
    pub active_edges: &'static str,

    pub snap_heading: &'static str,
    pub snap_description: &'static str,
    pub snap_active: &'static str,
    pub snap_radius: &'static str,
    pub active_snap_zones: &'static str,
    pub snap_hint: &'static str,
}

static DE: Strings = Strings {
    app_subtitle: "virtueller Monitorteiler (X11 / xrandr)",
    refresh: "Aktualisieren",
    language: "Sprache",

    physical_outputs: "Physische Ausgänge",
    disconnected: "disconnected",
    active_monitors: "Aktive Monitore",

    no_output_selected: "Kein aktiver Ausgang ausgewählt.",
    split_for: "Split für",
    native_prefix: "Nativ",
    position: "Position",

    split_count: "Anzahl Splits:",
    distribute_evenly: "gleich verteilen",
    ratios_normalized: "Verhältnisse (werden auf 1 normiert):",
    part: "Teil",
    preview: "Vorschau:",
    details: "Details der geplanten Splits",

    apply_split: "✓ Split anwenden",
    reset_all: "↺ Alle virtuellen Monitore entfernen",
    status_applied: "Splits angewendet für",
    status_removed_prefix: "",
    status_removed_suffix: "virtuelle Monitore entfernt.",
    error_prefix: "Fehler",

    dividers_heading: "Trennlinien-Overlay",
    dividers_description: "Zeigt an den Grenzen der virtuellen Monitore eine dünne farbige Linie \
        (transparent, immer im Vordergrund, klick-durchlässig). Rein visuell — verändert nichts \
        an den Splits selbst.",
    show_dividers: "Trennlinien anzeigen",
    width_px: "Breite (px):",
    opacity: "Deckkraft:",
    color: "Farbe:",
    color_cyan: "Cyan",
    color_red: "Rot",
    color_white: "Weiß",
    color_yellow: "Gelb",
    active_edges: "Aktive Grenzen",

    snap_heading: "Fenster-Andock",
    snap_description: "Beim Ziehen und Loslassen eines Fensters nahe einer virtuellen \
        Monitor-Grenze wird es auf die entsprechende Hälfte gesnapped.",
    snap_active: "Fenster-Andock aktiv",
    snap_radius: "Fangradius (px):",
    active_snap_zones: "Aktive Snap-Zonen",
    snap_hint: "Hinweis: das aktive Fenster ist entscheidend — vor dem Ziehen kurz auf den \
        Titel klicken, damit es fokussiert ist.",
};

static EN: Strings = Strings {
    app_subtitle: "virtual monitor splitter (X11 / xrandr)",
    refresh: "Refresh",
    language: "Language",

    physical_outputs: "Physical outputs",
    disconnected: "disconnected",
    active_monitors: "Active monitors",

    no_output_selected: "No active output selected.",
    split_for: "Split for",
    native_prefix: "Native",
    position: "Position",

    split_count: "Split count:",
    distribute_evenly: "distribute evenly",
    ratios_normalized: "Ratios (normalised to 1):",
    part: "Part",
    preview: "Preview:",
    details: "Planned split details",

    apply_split: "✓ Apply split",
    reset_all: "↺ Remove all virtual monitors",
    status_applied: "Splits applied for",
    status_removed_prefix: "Removed",
    status_removed_suffix: "virtual monitor(s).",
    error_prefix: "Error",

    dividers_heading: "Divider overlay",
    dividers_description: "Draws a thin coloured line at every virtual monitor boundary \
        (transparent, always on top, click-through). Purely visual — does not affect the \
        splits themselves.",
    show_dividers: "Show dividers",
    width_px: "Width (px):",
    opacity: "Opacity:",
    color: "Colour:",
    color_cyan: "Cyan",
    color_red: "Red",
    color_white: "White",
    color_yellow: "Yellow",
    active_edges: "Active boundaries",

    snap_heading: "Window snapping",
    snap_description: "When you drag and release a window near a virtual monitor boundary \
        it snaps to the matching half.",
    snap_active: "Window snapping enabled",
    snap_radius: "Snap radius (px):",
    active_snap_zones: "Active snap zones",
    snap_hint: "Note: the active window matters — click its title bar before dragging so \
        it is focused.",
};