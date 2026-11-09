use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn parse(val: &str) -> Option<Self> {
        let s = val.trim();
        if s.starts_with('#') {
            Self::from_hex(s)
        } else if s.starts_with("rgb(") || s.starts_with("rgba(") {
            Self::from_rgb_func(s)
        } else {
            Self::from_named_color(s)
        }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let clean = hex.trim().trim_start_matches('#');
        match clean.len() {
            3 => {
                let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).ok()?;
                Some(Color { r, g, b, a: 255 })
            }
            4 => {
                let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).ok()?;
                let a = u8::from_str_radix(&clean[3..4].repeat(2), 16).ok()?;
                Some(Color { r, g, b, a })
            }
            6 => {
                let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
                let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
                let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
                Some(Color { r, g, b, a: 255 })
            }
            8 => {
                let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
                let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
                let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
                let a = u8::from_str_radix(&clean[6..8], 16).ok()?;
                Some(Color { r, g, b, a })
            }
            _ => None,
        }
    }

    pub fn from_rgb_func(func: &str) -> Option<Self> {
        let inner = if let Some(stripped) = func.strip_prefix("rgba(") {
            stripped.strip_suffix(')')?
        } else if let Some(stripped) = func.strip_prefix("rgb(") {
            stripped.strip_suffix(')')?
        } else {
            return None;
        };

        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            Some(Color { r, g, b, a: 255 })
        } else if parts.len() == 4 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            let a_str = parts[3];
            let a = if a_str.ends_with('%') {
                let pct = a_str.trim_end_matches('%').parse::<f32>().ok()?;
                (pct * 2.55).clamp(0.0, 255.0) as u8
            } else if let Ok(val) = a_str.parse::<f32>() {
                if val <= 1.0 {
                    (val * 255.0).clamp(0.0, 255.0) as u8
                } else {
                    val.clamp(0.0, 255.0) as u8
                }
            } else {
                255
            };
            Some(Color { r, g, b, a })
        } else {
            None
        }
    }

    pub fn from_named_color(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "transparent" => Some(Color { r: 0, g: 0, b: 0, a: 0 }),
            "black" => Some(Color { r: 0, g: 0, b: 0, a: 255 }),
            "white" => Some(Color { r: 255, g: 255, b: 255, a: 255 }),
            "red" => Some(Color { r: 239, g: 68, b: 68, a: 255 }),
            "green" => Some(Color { r: 34, g: 197, b: 94, a: 255 }),
            "blue" => Some(Color { r: 59, g: 130, b: 246, a: 255 }),
            "yellow" => Some(Color { r: 234, g: 179, b: 8, a: 255 }),
            "cyan" => Some(Color { r: 6, g: 182, b: 212, a: 255 }),
            "magenta" | "purple" => Some(Color { r: 168, g: 85, b: 247, a: 255 }),
            "gray" | "grey" => Some(Color { r: 107, g: 114, b: 128, a: 255 }),
            "lightgray" | "lightgrey" => Some(Color { r: 229, g: 231, b: 235, a: 255 }),
            "darkgray" | "darkgrey" => Some(Color { r: 55, g: 65, b: 81, a: 255 }),
            "slate" => Some(Color { r: 100, g: 116, b: 139, a: 255 }),
            "zinc" => Some(Color { r: 113, g: 113, b: 122, a: 255 }),
            "indigo" => Some(Color { r: 99, g: 102, b: 241, a: 255 }),
            "emerald" => Some(Color { r: 16, g: 185, b: 129, a: 255 }),
            "rose" => Some(Color { r: 244, g: 63, b: 94, a: 255 }),
            "amber" => Some(Color { r: 245, g: 158, b: 11, a: 255 }),
            "orange" => Some(Color { r: 249, g: 115, b: 22, a: 255 }),
            _ => None,
        }
    }

    pub fn to_egui_code(&self) -> String {
        if self.a == 255 {
            format!("egui::Color32::from_rgb({}, {}, {})", self.r, self.g, self.b)
        } else if self.a == 0 {
            "egui::Color32::TRANSPARENT".to_string()
        } else {
            format!(
                "egui::Color32::from_rgba_premultiplied({}, {}, {}, {})",
                self.r, self.g, self.b, self.a
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Margin {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Margin {
    /// Strict unit parser: px / rem / em suffixes or unitless zero.
    /// Anything else (unknown units, garbage) is `None` so invalid
    /// declarations are ignored instead of silently becoming 0.
    fn parse_unit(s: &str) -> Option<f32> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        if let Some(num) = s.strip_suffix("px") {
            return num.parse::<f32>().ok();
        }
        if let Some(num) = s.strip_suffix("rem") {
            return num.parse::<f32>().ok().map(|v| v * 16.0);
        }
        if s.ends_with("em") && !s.ends_with("rem") {
            let num = s.strip_suffix("em")?;
            return num.parse::<f32>().ok().map(|v| v * 16.0);
        }
        // Unitless numbers: only zero is valid CSS without a unit.
        match s.parse::<f32>() {
            Ok(v) if v == 0.0 => Some(0.0),
            _ => None,
        }
    }

    pub fn parse(val: &str) -> Option<Self> {
        let parts: Vec<&str> = val.split_whitespace().collect();

        match parts.len() {
            1 => {
                let m = Self::parse_unit(parts[0])?;
                Some(Margin { top: m, right: m, bottom: m, left: m })
            }
            2 => {
                let y = Self::parse_unit(parts[0])?;
                let x = Self::parse_unit(parts[1])?;
                Some(Margin { top: y, right: x, bottom: y, left: x })
            }
            3 => {
                let top = Self::parse_unit(parts[0])?;
                let x = Self::parse_unit(parts[1])?;
                let bottom = Self::parse_unit(parts[2])?;
                Some(Margin { top, right: x, bottom, left: x })
            }
            4 => {
                let top = Self::parse_unit(parts[0])?;
                let right = Self::parse_unit(parts[1])?;
                let bottom = Self::parse_unit(parts[2])?;
                let left = Self::parse_unit(parts[3])?;
                Some(Margin { top, right, bottom, left })
            }
            _ => None,
        }
    }

    pub fn to_egui_code(&self) -> String {
        if (self.top - self.right).abs() < 0.001
            && (self.top - self.bottom).abs() < 0.001
            && (self.top - self.left).abs() < 0.001
        {
            format!("egui::Margin::same({:.1})", self.top)
        } else {
            format!(
                "egui::Margin {{ top: {:.1}, right: {:.1}, bottom: {:.1}, left: {:.1} }}",
                self.top, self.right, self.bottom, self.left
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}

impl Stroke {
    /// Parses `border` shorthand (`<width> <style-ignored> <color>`).
    /// Returns `None` when no usable width or color is found, so garbage
    /// like `border: nonsense` is ignored instead of becoming black.
    pub fn parse(val: &str) -> Option<Self> {
        let parts: Vec<&str> = val.split_whitespace().collect();
        let mut width = 1.0f32;
        let mut width_seen = false;
        let mut color = Color { r: 0, g: 0, b: 0, a: 255 };
        let mut color_seen = false;

        for p in parts {
            if let Some(num) = p.strip_suffix("rem") {
                if let Ok(w) = num.parse::<f32>() {
                    width = w * 16.0;
                    width_seen = true;
                }
            } else if let Some(num) = p.strip_suffix("em") {
                if let Ok(w) = num.parse::<f32>() {
                    width = w * 16.0;
                    width_seen = true;
                }
            } else if let Some(num) = p.strip_suffix("px") {
                if let Ok(w) = num.parse::<f32>() {
                    width = w;
                    width_seen = true;
                }
            } else if let Some(c) = Color::parse(p) {
                color = c;
                color_seen = true;
            }
        }
        if !width_seen && !color_seen {
            return None;
        }
        Some(Stroke { width, color })
    }

    pub fn to_egui_code(&self) -> String {
        format!("egui::Stroke::new({:.1}_f32, {})", self.width, self.color.to_egui_code())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CssRule {
    pub selector: String,
    pub properties: HashMap<String, String>,
}

/// Normalize a property name: `backgroundColor` (React inline-style form)
/// becomes `background-color`. Already-kebab names pass through.
pub fn normalize_prop_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Every property name the engine understands (parsed + mapped, or parsed
/// with a documented fallback). Used for `style={{...}}` validation.
pub fn known_properties() -> Vec<&'static str> {
    vec![
        "background", "background-color", "background-image", "background-position",
        "background-size", "background-repeat", "background-attachment",
        "background-blend-mode", "background-clip", "color", "font", "font-family",
        "font-size", "font-weight", "font-style", "font-variant", "line-height",
        "text-align", "text-decoration", "text-transform", "text-indent",
        "text-shadow", "text-overflow", "letter-spacing", "word-spacing",
        "white-space", "word-break", "overflow-wrap", "word-wrap", "writing-mode",
        "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
        "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
        "box-sizing", "aspect-ratio", "width", "min-width", "max-width", "height",
        "min-height", "max-height", "display", "float", "clear", "visibility",
        "overflow", "overflow-x", "overflow-y", "flex", "flex-direction",
        "flex-wrap", "flex-flow", "flex-grow", "flex-shrink", "flex-basis",
        "justify-content", "align-items", "align-content", "align-self", "gap",
        "row-gap", "column-gap", "order", "grid", "grid-template-columns",
        "grid-template-rows", "grid-template-areas", "grid-auto-columns",
        "grid-auto-rows", "grid-auto-flow", "grid-column", "grid-row", "grid-area",
        "justify-items", "justify-self", "position", "top", "right", "bottom",
        "left", "inset", "z-index", "border", "border-top", "border-right",
        "border-bottom", "border-left", "border-width", "border-style",
        "border-color", "border-radius", "outline", "outline-offset", "box-shadow",
        "transition", "transition-property", "transition-duration",
        "transition-timing-function", "transition-delay", "animation",
        "animation-name", "animation-duration", "animation-timing-function",
        "animation-delay", "animation-iteration-count", "animation-direction",
        "animation-fill-mode", "transform", "transform-origin", "opacity",
        "filter", "backdrop-filter", "clip-path", "mask", "cursor",
        "pointer-events", "user-select", "appearance", "resize", "list-style",
        "list-style-type", "list-style-position", "list-style-image",
        "border-collapse", "border-spacing", "content", "object-fit",
        "object-position", "scroll-behavior", "scroll-snap-type",
        "scroll-snap-align", "overscroll-behavior", "color-scheme",
    ]
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CssStyleSheet {
    pub rules: Vec<CssRule>,
}

impl CssStyleSheet {
    pub fn parse(css: &str) -> Self {
        let mut rules = Vec::new();
        let mut chars = css.chars().peekable();

        while chars.peek().is_some() {
            // Read until '{'
            let mut selector = String::new();
            while let Some(&c) = chars.peek() {
                if c == '{' {
                    chars.next();
                    break;
                }
                selector.push(c);
                chars.next();
            }

            let sel = selector.trim().to_string();
            if sel.is_empty() {
                break;
            }

            // Read properties until '}'
            let mut block_body = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next();
                    break;
                }
                block_body.push(c);
                chars.next();
            }

            let mut properties = HashMap::new();
            for line in block_body.split(';') {
                let trimmed = line.trim();
                if let Some(colon_pos) = trimmed.find(':') {
                    let key = trimmed[..colon_pos].trim().to_lowercase();
                    let val = trimmed[colon_pos + 1..].trim().to_string();
                    if !key.is_empty() {
                        properties.insert(key, val);
                    }
                }
            }

            rules.push(CssRule {
                selector: sel,
                properties,
            });
        }

        Self { rules }
    }

    /// Merge CSS rules that match a list of class names (e.g. `["btn_filter", "aktif"]`)
    pub fn resolve_properties(&self, classes: &[&str]) -> HashMap<String, String> {
        let mut merged = HashMap::new();

        for rule in &self.rules {
            let sel = rule.selector.trim_start_matches('.');
            let sel_classes: Vec<&str> = sel.split('.').collect();

            // Check if all classes in the selector are present in classes
            let matches = sel_classes.iter().all(|sc| classes.contains(sc));
            if matches {
                for (k, v) in &rule.properties {
                    merged.insert(k.clone(), v.clone());
                }
            }
        }

        merged
    }

    /// Generates code for egui::Frame for container tags (<Panel>, <Row>, <Col>, etc.)
    pub fn get_frame_code(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        if props.is_empty() {
            return None;
        }

        let mut frame_calls = Vec::new();

        // Opacity 0..1 — modulate fill alpha (egui 0.27 Frame has no opacity field).
        let opacity = Self::parse_opacity(props.get("opacity").map(|s| s.as_str()).unwrap_or("1"));

        // Background / background-color
        if let Some(bg) = props.get("background").or_else(|| props.get("background-color")) {
            if let Some(c) = Color::parse(bg) {
                let c = if opacity < 1.0 {
                    let a = ((c.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                    Color::new(c.r, c.g, c.b, a)
                } else {
                    c
                };
                frame_calls.push(format!("fill({})", c.to_egui_code()));
            }
        }

        // Padding -> inner_margin
        if let Some(pad) = props.get("padding") {
            if let Some(m) = Margin::parse(pad) {
                frame_calls.push(format!("inner_margin({})", m.to_egui_code()));
            }
        }

        // Margin -> outer_margin (egui 0.27 Frame supports outer_margin)
        if let Some(margin_val) = props.get("margin") {
            if let Some(m) = Margin::parse(margin_val) {
                frame_calls.push(format!("outer_margin({})", m.to_egui_code()));
            }
        }

        // Border radius
        if let Some(br) = props.get("border-radius") {
            let r = Self::parse_size(br).unwrap_or(0.0);
            frame_calls.push(format!("rounding(egui::Rounding::same({:.1}))", r));
        }

        // Border / stroke (border, or border-width + border-color).
        // Stroke color is opacity-modulated, like fill, for consistency.
        if let Some(border) = props.get("border") {
            if let Some(mut s) = Stroke::parse(border) {
                if opacity < 1.0 {
                    let a = ((s.color.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                    s.color.a = a;
                }
                frame_calls.push(format!("stroke({})", s.to_egui_code()));
            }
        } else if let Some(width_str) = props.get("border-width") {
            let w = Self::parse_size(width_str).unwrap_or(1.0);
            let mut color = if let Some(c_str) = props.get("border-color") {
                Color::parse(c_str).unwrap_or(Color::rgb(0, 0, 0))
            } else {
                Color::rgb(0, 0, 0)
            };
            if opacity < 1.0 {
                let a = ((color.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                color = Color::new(color.r, color.g, color.b, a);
            }
            frame_calls.push(format!("stroke(egui::Stroke::new({:.1}_f32, {}))", w, color.to_egui_code()));
        } else if let Some(color_str) = props.get("border-color") {
            if let Some(mut c) = Color::parse(color_str) {
                if opacity < 1.0 {
                    let a = ((c.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                    c = Color::new(c.r, c.g, c.b, a);
                }
                frame_calls.push(format!("stroke(egui::Stroke::new(1.0_f32, {}))", c.to_egui_code()));
            }
        }

        // Shadow
        if let Some(shadow_val) = props.get("box-shadow") {
            if shadow_val == "none" {
                frame_calls.push("shadow(egui::epaint::Shadow::NONE)".to_string());
            }
        }

        if frame_calls.is_empty() {
            None
        } else {
            let chain = frame_calls.join(".");
            Some(format!("egui::Frame::none().{chain}"))
        }
    }

    /// `fill(...)` fragment for `background` / `background-color` (opacity-modulated).
    pub fn get_fill_code(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        let opacity = Self::parse_opacity(props.get("opacity").map(|s| s.as_str()).unwrap_or("1"));
        let bg = props
            .get("background")
            .or_else(|| props.get("background-color"))?;
        let mut c = Color::parse(bg)?;
        if opacity < 1.0 {
            let a = ((c.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
            c = Color::new(c.r, c.g, c.b, a);
        }
        Some(format!("fill({})", c.to_egui_code()))
    }

    /// `stroke(...)` fragment for `border` / `border-width` + `border-color`.
    /// Stroke color is opacity-modulated, like fill, for consistency.
    pub fn get_stroke_code(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        let opacity = Self::parse_opacity(props.get("opacity").map(|s| s.as_str()).unwrap_or("1"));
        if let Some(border) = props.get("border") {
            let mut s = Stroke::parse(border)?;
            if opacity < 1.0 {
                let a = ((s.color.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                s.color.a = a;
            }
            return Some(format!("stroke({})", s.to_egui_code()));
        }
        if let Some(width_str) = props.get("border-width") {
            let w = Self::parse_size(width_str).unwrap_or(1.0);
            let mut color = props
                .get("border-color")
                .and_then(|s| Color::parse(s.as_str()))
                .unwrap_or(Color::rgb(0, 0, 0));
            if opacity < 1.0 {
                let a = ((color.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                color = Color::new(color.r, color.g, color.b, a);
            }
            return Some(format!(
                "stroke(egui::Stroke::new({:.1}_f32, {}))",
                w,
                color.to_egui_code()
            ));
        }
        if let Some(color_str) = props.get("border-color") {
            if let Some(mut c) = Color::parse(color_str) {
                if opacity < 1.0 {
                    let a = ((c.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                    c = Color::new(c.r, c.g, c.b, a);
                }
                return Some(format!(
                    "stroke(egui::Stroke::new(1.0_f32, {}))",
                    c.to_egui_code()
                ));
            }
        }
        None
    }

    /// `rounding(...)` fragment for `border-radius`.
    pub fn get_rounding_code(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        let br = props.get("border-radius")?;
        let r = Self::parse_size(br).unwrap_or(0.0);
        Some(format!("rounding(egui::Rounding::same({:.1}))", r))
    }

    /// Numeric `font-size` in px (units rem/em/px supported).
    pub fn get_font_size(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("font-size").and_then(|fs| Self::parse_size(fs))
    }

    /// Text `color` as an `egui::Color32` expression (opacity-modulated).
    pub fn get_text_color_code(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        let opacity = Self::parse_opacity(props.get("opacity").map(|s| s.as_str()).unwrap_or("1"));
        let mut c = Color::parse(props.get("color")?)?;
        if opacity < 1.0 {
            let a = ((c.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
            c = Color::new(c.r, c.g, c.b, a);
        }
        Some(c.to_egui_code())
    }

    /// Generates code for egui::RichText modifiers
    pub fn get_rich_text_modifiers(&self, classes: &[&str]) -> Vec<String> {
        let props = self.resolve_properties(classes);
        let mut mods = Vec::new();

        let opacity = Self::parse_opacity(props.get("opacity").map(|s| s.as_str()).unwrap_or("1"));
        if let Some(color_val) = props.get("color") {
            if let Some(c) = Color::parse(color_val) {
                let c = if opacity < 1.0 {
                    let a = ((c.a as f32 * opacity).clamp(0.0, 255.0)) as u8;
                    Color::new(c.r, c.g, c.b, a)
                } else {
                    c
                };
                mods.push(format!("color({})", c.to_egui_code()));
            }
        }

        if let Some(fs) = props.get("font-size") {
            let size = Self::parse_size(fs).unwrap_or(14.0);
            mods.push(format!("size({:.1})", size));
        }

        if let Some(fw) = props.get("font-weight") {
            if fw == "bold" || fw == "700" || fw == "800" || fw == "900" || fw == "bolder" {
                mods.push("strong()".to_string());
            }
        }

        if let Some(fst) = props.get("font-style") {
            if fst == "italic" || fst == "oblique" {
                mods.push("italics()".to_string());
            }
        }

        if let Some(td) = props.get("text-decoration") {
            if td.contains("underline") {
                mods.push("underline()".to_string());
            }
            if td.contains("line-through") {
                mods.push("strikethrough()".to_string());
            }
        }

        if let Some(ff) = props.get("font-family") {
            if ff.contains("monospace") || ff.contains("Courier") || ff.contains("Consolas") {
                mods.push("monospace()".to_string());
            }
        }

        mods
    }

    pub fn get_spacing(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        if let Some(gap) = props
            .get("gap")
            .or_else(|| props.get("row-gap"))
            .or_else(|| props.get("column-gap"))
        {
            Self::parse_size(gap)
        } else {
            None
        }
    }

    pub fn get_max_height(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        if let Some(h) = props.get("height").or_else(|| props.get("max-height")) {
            Self::parse_size(h)
        } else {
            None
        }
    }

    pub fn get_max_width(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        if let Some(w) = props.get("width").or_else(|| props.get("max-width")) {
            Self::parse_size(w)
        } else {
            None
        }
    }

    /// Exact `width` property (falls back to None if only max-width is set).
    pub fn get_width(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("width").and_then(|w| Self::parse_size(w))
    }

    /// Exact `height` property (falls back to None if only max-height is set).
    pub fn get_height(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("height").and_then(|h| Self::parse_size(h))
    }

    /// `opacity: 0..1` (defaults to None when unset).
    pub fn get_opacity(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props
            .get("opacity")
            .and_then(|v| v.trim().parse::<f32>().ok())
            .map(|o| o.clamp(0.0, 1.0))
    }

    /// `align-items` (flex cross-axis alignment hint).
    pub fn get_align_items(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("align-items").cloned().or_else(|| props.get("align_items").cloned())
    }

    /// `justify-content` (flex main-axis alignment hint).
    pub fn get_justify_content(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props
            .get("justify-content")
            .cloned()
            .or_else(|| props.get("justify_content").cloned())
    }

    /// Raw property lookup (lowercased name). Parsed but not necessarily wired
    /// to codegen — used to document fallback properties (ignored by emit).
    pub fn get_raw_property(&self, classes: &[&str], name: &str) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get(&name.to_lowercase()).cloned()
    }

    /// `min-width` in px (parsed but currently ignored by codegen; fallback).
    pub fn get_min_width(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("min-width").and_then(|v| Self::parse_size(v))
    }

    /// `min-height` in px (parsed but currently ignored by codegen; fallback).
    pub fn get_min_height(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("min-height").and_then(|v| Self::parse_size(v))
    }

    /// Margin longhands in px (parsed but currently ignored by `get_frame_code`,
    /// which only reads the `margin` shorthand; fallback: ignored).
    pub fn get_margin_top(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("margin-top").and_then(|v| Self::parse_size(v))
    }
    pub fn get_margin_right(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("margin-right").and_then(|v| Self::parse_size(v))
    }
    pub fn get_margin_bottom(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("margin-bottom").and_then(|v| Self::parse_size(v))
    }
    pub fn get_margin_left(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("margin-left").and_then(|v| Self::parse_size(v))
    }

    /// Padding longhands in px (parsed but currently ignored by `get_frame_code`,
    /// which only reads the `padding` shorthand; fallback: ignored).
    pub fn get_padding_top(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("padding-top").and_then(|v| Self::parse_size(v))
    }
    pub fn get_padding_right(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("padding-right").and_then(|v| Self::parse_size(v))
    }
    pub fn get_padding_bottom(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("padding-bottom").and_then(|v| Self::parse_size(v))
    }
    pub fn get_padding_left(&self, classes: &[&str]) -> Option<f32> {
        let props = self.resolve_properties(classes);
        props.get("padding-left").and_then(|v| Self::parse_size(v))
    }

    /// Effective margin: `margin` shorthand with longhand overrides applied.
    /// Parsed for future wiring; codegen currently uses only the shorthand.
    pub fn get_effective_margin(&self, classes: &[&str]) -> Option<Margin> {
        let props = self.resolve_properties(classes);
        let mut base = props.get("margin").and_then(|v| Margin::parse(v))?;
        if let Some(v) = props.get("margin-top").and_then(|v| Self::parse_size(v)) {
            base.top = v;
        }
        if let Some(v) = props.get("margin-right").and_then(|v| Self::parse_size(v)) {
            base.right = v;
        }
        if let Some(v) = props.get("margin-bottom").and_then(|v| Self::parse_size(v)) {
            base.bottom = v;
        }
        if let Some(v) = props.get("margin-left").and_then(|v| Self::parse_size(v)) {
            base.left = v;
        }
        Some(base)
    }

    /// Effective padding: `padding` shorthand with longhand overrides applied.
    /// Parsed for future wiring; codegen currently uses only the shorthand.
    pub fn get_effective_padding(&self, classes: &[&str]) -> Option<Margin> {
        let props = self.resolve_properties(classes);
        let mut base = props.get("padding").and_then(|v| Margin::parse(v))?;
        if let Some(v) = props.get("padding-top").and_then(|v| Self::parse_size(v)) {
            base.top = v;
        }
        if let Some(v) = props.get("padding-right").and_then(|v| Self::parse_size(v)) {
            base.right = v;
        }
        if let Some(v) = props.get("padding-bottom").and_then(|v| Self::parse_size(v)) {
            base.bottom = v;
        }
        if let Some(v) = props.get("padding-left").and_then(|v| Self::parse_size(v)) {
            base.left = v;
        }
        Some(base)
    }

    /// `display` raw value (parsed, ignored by codegen; fallback).
    pub fn get_display(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("display").cloned()
    }

    /// `position` raw value (parsed, ignored by codegen; fallback).
    pub fn get_position(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("position").cloned()
    }

    /// `visibility` raw value (parsed, ignored by codegen; fallback).
    pub fn get_visibility(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("visibility").cloned()
    }

    /// `overflow` (or `overflow-x`/`overflow-y` if `overflow` unset).
    /// Parsed, ignored by codegen; Scroll always uses vertical area.
    pub fn get_overflow(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props
            .get("overflow")
            .cloned()
            .or_else(|| props.get("overflow-x").cloned())
            .or_else(|| props.get("overflow-y").cloned())
    }

    /// `box-sizing` raw value (parsed, ignored by codegen; fallback).
    pub fn get_box_sizing(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("box-sizing").cloned()
    }

    /// `aspect-ratio` raw value (parsed, ignored by codegen; fallback).
    pub fn get_aspect_ratio(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("aspect-ratio").cloned()
    }

    /// `text-align` raw value (parsed, ignored by codegen; fallback).
    pub fn get_text_align(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("text-align").cloned()
    }

    /// `flex-direction` raw value (parsed, ignored by codegen; fallback).
    pub fn get_flex_direction(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("flex-direction").cloned()
    }

    /// `flex-wrap` raw value (parsed, ignored by codegen; fallback).
    pub fn get_flex_wrap(&self, classes: &[&str]) -> Option<String> {
        let props = self.resolve_properties(classes);
        props.get("flex-wrap").cloned()
    }

    /// Parse a CSS length into logical pixels. Supports px, pt, %, rem, em and bare numbers.
    pub fn parse_size(val: &str) -> Option<f32> {
        let s = val.trim().to_lowercase();
        // Take first token (e.g. "6px 14px" -> "6px") for robustness.
        let tok = s.split_whitespace().next().unwrap_or("").trim();
        if tok.is_empty() || tok == "auto" || tok == "none" {
            return None;
        }
        if let Some(num) = tok.strip_suffix("px") {
            return num.trim().parse::<f32>().ok();
        }
        if let Some(num) = tok.strip_suffix("pt") {
            return num.trim().parse::<f32>().ok();
        }
        if let Some(num) = tok.strip_suffix("rem") {
            return num.trim().parse::<f32>().ok().map(|v| v * 16.0);
        }
        if let Some(num) = tok.strip_suffix("em") {
            return num.trim().parse::<f32>().ok().map(|v| v * 16.0);
        }
        if let Some(num) = tok.strip_suffix('%') {
            // Percentages have no absolute meaning in egui; treat as unset.
            let _ = num.trim().parse::<f32>().ok()?;
            return None;
        }
        tok.parse::<f32>().ok()
    }

    fn parse_opacity(val: &str) -> f32 {
        val.trim()
            .trim_end_matches('%')
            .parse::<f32>()
            .map(|o| {
                if val.trim().ends_with('%') {
                    (o / 100.0).clamp(0.0, 1.0)
                } else {
                    o.clamp(0.0, 1.0)
                }
            })
            .unwrap_or(1.0)
    }
}
