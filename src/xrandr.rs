use std::process::Command;

#[derive(Debug, Clone)]
pub struct Output {
    pub name: String,
    pub connected: bool,
    pub width_px: u32,
    pub height_px: u32,
    pub width_mm: u32,
    pub height_mm: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub width_px: u32,
    pub height_px: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
    pub is_virtual: bool,
    pub backing_output: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SplitSpec {
    pub name: String,
    pub width_px: u32,
    pub height_px: u32,
    pub width_mm: u32,
    pub height_mm: u32,
    pub x: i32,
    pub y: i32,
}

pub fn list_outputs() -> anyhow::Result<Vec<Output>> {
    let out = Command::new("xrandr").arg("--query").output()?;
    if !out.status.success() {
        anyhow::bail!("xrandr --query fehlgeschlagen");
    }
    parse_query(std::str::from_utf8(&out.stdout)?)
}

pub fn list_monitors() -> anyhow::Result<Vec<Monitor>> {
    let out = Command::new("xrandr").arg("--listmonitors").output()?;
    if !out.status.success() {
        anyhow::bail!("xrandr --listmonitors fehlgeschlagen");
    }
    parse_listmonitors(std::str::from_utf8(&out.stdout)?)
}

pub fn apply_split(output_name: &str, splits: &[SplitSpec]) -> anyhow::Result<()> {
    if splits.is_empty() {
        anyhow::bail!("keine Splits übergeben");
    }

    // Vorhandene virtuelle Monitore auf demselben Output vorher entfernen,
    // damit "--setmonitor" nicht mit Duplikaten kollidiert.
    if let Ok(current) = list_monitors() {
        for m in current {
            if m.is_virtual && m.backing_output.as_deref() == Some(output_name) {
                let _ = Command::new("xrandr").args(["--delmonitor", &m.name]).status();
            }
        }
    }

    for (i, s) in splits.iter().enumerate() {
        let backing = if i == 0 { output_name } else { "none" };
        let geom = format!(
            "{}/{}x{}/{}+{}+{}",
            s.width_px, s.width_mm, s.height_px, s.height_mm, s.x, s.y
        );
        let status = Command::new("xrandr")
            .args(["--setmonitor", &s.name, &geom, backing])
            .status()?;
        if !status.success() {
            anyhow::bail!("xrandr --setmonitor {} fehlgeschlagen", s.name);
        }
    }
    Ok(())
}

pub fn remove_all_virtual() -> anyhow::Result<usize> {
    let monitors = list_monitors()?;
    let mut removed = 0;
    for m in monitors {
        if m.is_virtual {
            let status = Command::new("xrandr")
                .args(["--delmonitor", &m.name])
                .status()?;
            if status.success() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

fn parse_query(s: &str) -> anyhow::Result<Vec<Output>> {
    let mut outs = Vec::new();
    for line in s.lines() {
        // Ausgabe von Modeline-Zeilen (mit whitespace beginnend) und Screen-Header ignorieren
        if line.starts_with(' ') || line.starts_with('\t') || line.starts_with("Screen ") {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(status) = parts.next() else { continue };

        if status == "disconnected" {
            outs.push(Output {
                name: name.to_string(),
                connected: false,
                width_px: 0,
                height_px: 0,
                width_mm: 0,
                height_mm: 0,
                x: 0,
                y: 0,
            });
            continue;
        }
        if status != "connected" {
            continue;
        }

        // Nächstes Token kann "primary" oder direkt die Geometrie sein
        let mut geom_str = parts.next().unwrap_or("");
        if geom_str == "primary" {
            geom_str = parts.next().unwrap_or("");
        }

        // Geometry-Format: "WWWWxHHHH+X+Y"
        let Some((w_s, rest)) = geom_str.split_once('x') else { continue };
        let Some(plus_idx) = rest.find('+') else { continue };
        let h_s = &rest[..plus_idx];
        let pos = &rest[plus_idx + 1..];
        let Some((x_s, y_s)) = pos.split_once('+') else { continue };

        // Physische Abmessungen am Zeilenende: "WWWmm x HHHmm"
        let (mut width_mm, mut height_mm) = (0u32, 0u32);
        let tokens: Vec<&str> = line.split_whitespace().collect();
        for (i, tok) in tokens.iter().enumerate() {
            if *tok == "x" && i > 0 && i + 1 < tokens.len() {
                let a = tokens[i - 1].strip_suffix("mm");
                let b = tokens[i + 1].strip_suffix("mm");
                if let (Some(a), Some(b)) = (a, b) {
                    width_mm = a.parse().unwrap_or(0);
                    height_mm = b.parse().unwrap_or(0);
                }
            }
        }

        outs.push(Output {
            name: name.to_string(),
            connected: true,
            width_px: w_s.parse().unwrap_or(0),
            height_px: h_s.parse().unwrap_or(0),
            width_mm,
            height_mm,
            x: x_s.parse().unwrap_or(0),
            y: y_s.parse().unwrap_or(0),
        });
    }
    Ok(outs)
}

fn parse_listmonitors(s: &str) -> anyhow::Result<Vec<Monitor>> {
    // Format:
    //   Monitors: 2
    //    0: +*HDMI-A-1 2560/597x1440/336+0+0  HDMI-A-1
    //    1: +HDMI-A-0 1920/1280x1080/720+2560+0  HDMI-A-0
    let mut monitors = Vec::new();
    for line in s.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let rest = line.splitn(2, ':').nth(1).unwrap_or("").trim();
        let mut parts = rest.split_whitespace();
        let Some(flag_name) = parts.next() else { continue };
        let Some(geometry) = parts.next() else { continue };
        let backing = parts.next().map(String::from);

        let mut is_primary = false;
        let mut chars = flag_name.chars().peekable();
        while let Some(&c) = chars.peek() {
            match c {
                '+' => { chars.next(); }
                '*' => { is_primary = true; chars.next(); }
                _ => break,
            }
        }
        let name: String = chars.collect();

        // "2560/597x1440/336+0+0"
        let Some((w_part, tail)) = geometry.split_once('x') else { continue };
        let Some((width_px_s, _width_mm_s)) = w_part.split_once('/') else { continue };
        let Some(plus_idx) = tail.find('+') else { continue };
        let h_part = &tail[..plus_idx];
        let pos_part = &tail[plus_idx + 1..];
        let Some((height_px_s, _height_mm_s)) = h_part.split_once('/') else { continue };
        let Some((x_s, y_s)) = pos_part.split_once('+') else { continue };

        let is_virtual = name.contains('~');

        monitors.push(Monitor {
            name,
            width_px: width_px_s.parse().unwrap_or(0),
            height_px: height_px_s.parse().unwrap_or(0),
            x: x_s.parse().unwrap_or(0),
            y: y_s.parse().unwrap_or(0),
            is_primary,
            is_virtual,
            backing_output: backing,
        });
    }
    Ok(monitors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listmonitors_sample() {
        let sample = "Monitors: 2\n \
                      0: +*HDMI-A-1 2560/597x1440/336+0+0  HDMI-A-1\n \
                      1: +HDMI-A-0 1920/1280x1080/720+2560+0  HDMI-A-0\n";
        let m = parse_listmonitors(sample).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].name, "HDMI-A-1");
        assert!(m[0].is_primary);
        assert_eq!(m[0].width_px, 2560);
        assert_eq!(m[1].x, 2560);
    }

    #[test]
    fn parses_query_sample() {
        let sample = "Screen 0: minimum 320 x 200, current 4480 x 1440, maximum 16384 x 16384\n\
                      DisplayPort-0 disconnected (normal left inverted right x axis y axis)\n\
                      HDMI-A-1 connected primary 2560x1440+0+0 (normal left inverted right x axis y axis) 597mm x 336mm\n   \
                      2560x1440     60.00 + 144.00*  120.00\n\
                      HDMI-A-0 connected 1920x1080+2560+0 (normal left inverted right x axis y axis) 1280mm x 720mm\n";
        let o = parse_query(sample).unwrap();
        let by_name: std::collections::HashMap<_, _> = o.iter().map(|o| (o.name.as_str(), o)).collect();
        assert!(!by_name["DisplayPort-0"].connected);
        assert_eq!(by_name["HDMI-A-1"].width_px, 2560);
        assert_eq!(by_name["HDMI-A-1"].width_mm, 597);
        assert_eq!(by_name["HDMI-A-0"].x, 2560);
    }

    #[test]
    fn detects_virtual_by_tilde() {
        let sample = "Monitors: 1\n 0: +UW~L 2560/175x1440/336+0+0  HDMI-A-1\n";
        let m = parse_listmonitors(sample).unwrap();
        assert!(m[0].is_virtual);
        assert_eq!(m[0].backing_output.as_deref(), Some("HDMI-A-1"));
    }
}