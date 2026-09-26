use super::super::effects::{AppearanceStack, DropShadow, GlowEffect};
use super::WidthProfile;
use crate::core::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "Color Dodge",
            Self::ColorBurn => "Color Burn",
            Self::HardLight => "Hard Light",
            Self::SoftLight => "Soft Light",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
        }
    }

    pub fn as_svg_str(&self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Multiply => Some("multiply"),
            Self::Screen => Some("screen"),
            Self::Overlay => Some("overlay"),
            Self::Darken => Some("darken"),
            Self::Lighten => Some("lighten"),
            Self::ColorDodge => Some("color-dodge"),
            Self::ColorBurn => Some("color-burn"),
            Self::HardLight => Some("hard-light"),
            Self::SoftLight => Some("soft-light"),
            Self::Difference => Some("difference"),
            Self::Exclusion => Some("exclusion"),
            Self::Hue => Some("hue"),
            Self::Saturation => Some("saturation"),
            Self::Color => Some("color"),
            Self::Luminosity => Some("luminosity"),
        }
    }

    /// Map onto the shared Photoshop-blend implementation in
    /// [`crate::core::blend`].
    ///
    /// The document keeps its own (serialized) copy of the mode list; the
    /// maths lives once in `core::blend` so the canvas preview, the CLI's
    /// `blend` command and the exporters all agree.  `core::blend` knows extra
    /// modes the document cannot express yet (Linear Dodge, Hard Mix, ...),
    /// which is why this is a mapping rather than a re-export.
    pub fn to_blend(self) -> crate::core::blend::BlendMode {
        use crate::core::blend::BlendMode as B;

        match self {
            Self::Normal => B::Normal,
            Self::Multiply => B::Multiply,
            Self::Screen => B::Screen,
            Self::Overlay => B::Overlay,
            Self::Darken => B::Darken,
            Self::Lighten => B::Lighten,
            Self::ColorDodge => B::ColorDodge,
            Self::ColorBurn => B::ColorBurn,
            Self::HardLight => B::HardLight,
            Self::SoftLight => B::SoftLight,
            Self::Difference => B::Difference,
            Self::Exclusion => B::Exclusion,
            Self::Hue => B::Hue,
            Self::Saturation => B::Saturation,
            Self::Color => B::Color,
            Self::Luminosity => B::Luminosity,
        }
    }

    pub fn all() -> &'static [BlendMode] {
        &[
            Self::Normal,
            Self::Multiply,
            Self::Screen,
            Self::Overlay,
            Self::Darken,
            Self::Lighten,
            Self::ColorDodge,
            Self::ColorBurn,
            Self::HardLight,
            Self::SoftLight,
            Self::Difference,
            Self::Exclusion,
            Self::Hue,
            Self::Saturation,
            Self::Color,
            Self::Luminosity,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    pub fn as_svg_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
            Self::Oblique => "oblique",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextAnchor {
    #[default]
    Start,
    Middle,
    End,
}

/// Rectangular text container (Illustrator area-type) in the object's local
/// coordinates: `x`/`y` is the top-left, `width`/`height` the box size.
/// The first baseline sits at `y + font_size` (em-box convention shared by
/// canvas, export and measurement).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TextArea {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl TextArea {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(1.0),
            height: height.max(1.0),
        }
    }
}

impl TextAnchor {
    pub fn as_svg_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }
}

fn default_font_family() -> String {
    "Inter, sans-serif".to_string()
}

fn default_font_size() -> f64 {
    24.0
}

fn default_font_weight() -> u16 {
    400
}

/// One OpenType variation axis coordinate (variable fonts).
/// `axis` is the 4-char tag (`wght`, `wdth`, `opsz`, …); `value` is in the
/// axis's design space (e.g. 100..=900 for `wght`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariationSetting {
    pub axis: String,
    pub value: f64,
}

impl VariationSetting {
    pub fn new(axis: impl Into<String>, value: f64) -> Self {
        Self {
            axis: axis.into(),
            value,
        }
    }

    /// True when this entry differs from the axis default (so we can skip
    /// no-op coordinates when applying to a face).
    pub fn is_non_default(&self, def: f64) -> bool {
        (self.value - def).abs() > f64::EPSILON
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default = "default_font_weight")]
    pub font_weight: u16,
    #[serde(default)]
    pub font_style: FontStyle,
    #[serde(default)]
    pub letter_spacing: f64,
    #[serde(default)]
    pub text_anchor: TextAnchor,
    /// Line height multiplier.  `None` means the default 1.2× font size.
    #[serde(default)]
    pub line_height: Option<f64>,
    /// Maximum width before word-wrapping kicks in.  `None` means no wrap
    /// (legacy single-line / explicit `\n` behaviour).
    #[serde(default)]
    pub max_width: Option<f64>,
    /// Enable soft word wrapping at `max_width`.
    #[serde(default)]
    pub word_wrap: bool,
    /// Variable-font axis coordinates. Empty for static faces.
    #[serde(default)]
    pub variations: Vec<VariationSetting>,
    /// `true` = vertical writing (縦組み): columns advance right→left,
    /// each "line" is a column stacked on X instead of Y.
    #[serde(default)]
    pub vertical: bool,
    /// Enable GSUB `liga`/`dlig`/`clig`/`rlig` ligature substitution
    /// when the face provides them (outline path only; SVG keeps raw text).
    #[serde(default)]
    pub ligatures: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            font_weight: default_font_weight(),
            font_style: FontStyle::Normal,
            letter_spacing: 0.0,
            text_anchor: TextAnchor::Start,
            line_height: None,
            max_width: None,
            word_wrap: false,
            variations: Vec::new(),
            vertical: false,
            ligatures: true,
        }
    }
}

impl TextStyle {
    pub fn new(font_family: impl Into<String>, font_size: f64) -> Self {
        Self {
            font_family: font_family.into(),
            font_size,
            font_weight: 400,
            font_style: FontStyle::Normal,
            letter_spacing: 0.0,
            text_anchor: TextAnchor::Start,
            line_height: None,
            max_width: None,
            word_wrap: false,
            variations: Vec::new(),
            vertical: false,
            ligatures: true,
        }
    }

    /// Look up a stored variation coordinate by 4-char axis tag.
    pub fn variation(&self, axis: &str) -> Option<f64> {
        self.variations
            .iter()
            .find(|v| v.axis.eq_ignore_ascii_case(axis))
            .map(|v| v.value)
    }

    /// Insert or replace a variation coordinate (undo is the caller's job).
    pub fn set_variation(&mut self, axis: impl Into<String>, value: f64) {
        let axis = axis.into();
        if let Some(entry) = self
            .variations
            .iter_mut()
            .find(|v| v.axis.eq_ignore_ascii_case(&axis))
        {
            entry.value = value;
        } else {
            self.variations.push(VariationSetting::new(axis, value));
        }
    }

    /// Remove a variation coordinate if present.
    pub fn clear_variation(&mut self, axis: &str) {
        self.variations
            .retain(|v| !v.axis.eq_ignore_ascii_case(axis));
    }

    /// Serialize for SVG `font-variation-settings` / CSS round-trip.
    pub fn variation_settings_css(&self) -> String {
        self.variations
            .iter()
            .map(|v| format!("\"{}\" {}", v.axis, v.value))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Parse `font-variation-settings` content (`"wght" 700, "wdth" 100`).
    pub fn parse_variation_settings_css(s: &str) -> Vec<VariationSetting> {
        let mut out = Vec::new();
        for part in s.split(',') {
            let tokens: Vec<&str> = part.split_whitespace().collect();
            if tokens.len() < 2 {
                continue;
            }
            let tag = tokens[0].trim_matches(|c| c == '"' || c == '\'');
            if tag.len() != 4 {
                continue;
            }
            if let Ok(value) = tokens[1].parse::<f64>() {
                out.push(VariationSetting::new(tag, value));
            }
        }
        out
    }

    /// Effective line height in document units.
    pub fn effective_line_height(&self) -> f64 {
        self.line_height.unwrap_or(1.2) * self.font_size
    }
}

/// Which side of a path Text-on-Path glyphs sit on: `Top` follows the curve
/// direction; `Bottom` flips each glyph 180° around its arc anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextPathSide {
    #[default]
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectType {
    Path(PathData),
    Rectangle {
        width: f64,
        height: f64,
        corner_radius: f64,
    },
    Ellipse {
        rx: f64,
        ry: f64,
    },
    Star {
        points: usize,
        inner_radius: f64,
        outer_radius: f64,
    },
    Polygon {
        sides: usize,
        radius: f64,
    },
    Line {
        x2: f64,
        y2: f64,
    },
    Text {
        text: String,
        #[serde(default = "default_font_size")]
        font_size: f64,
        #[serde(default)]
        style: TextStyle,
        /// Area-type container. `None` = point text (legacy behaviour).
        #[serde(default)]
        area: Option<TextArea>,
        /// Next frame in a threaded text story (`None` = unthreaded).
        /// Only meaningful when `area` is `Some`.
        #[serde(default)]
        next_frame: Option<String>,
    },
    Group(Vec<Object>),
    ClippingMask {
        children: Vec<Object>,
    },
    Use {
        href: String,
        width: Option<f64>,
        height: Option<f64>,
    },
    /// Placed raster image (PNG bytes, embedded). Local origin at top-left,
    /// `width`/`height` in document units; positioned by `transform`.
    Image {
        width: f64,
        height: f64,
        png_bytes: Vec<u8>,
    },
    /// Pixel-art layer (dot絵): fixed grid of palette indices, 1 cell = 1
    /// local unit. See [`crate::core::pixel::PixelArt`].
    PixelArt(crate::core::pixel::PixelArt),
    /// Gradient mesh: editable grid of colored nodes forming smooth
    /// Hermite patches. See [`crate::core::gradient_mesh::MeshGradient`].
    GradientMesh(crate::core::gradient_mesh::MeshGradient),
    /// Live envelope: `source` (normalized to a Path with identity
    /// transform at wrap time) deformed by kind/amount on every read.
    /// Edit params or release to restore the source.
    /// Live text laid out along a path. `path` is a self-contained local-space
    /// copy; `source_path_id` optionally records the sibling Path this was
    /// created from (stored as a link only — no live re-sync in v1).
    /// Glyph outlines are recomputed on every read via
    /// [`crate::core::text_path::text_on_path_outlines`].
    TextOnPath {
        text: String,
        #[serde(default)]
        style: TextStyle,
        path: PathData,
        #[serde(default)]
        source_path_id: Option<String>,
        /// Arc-length distance along `path` where the first glyph starts.
        #[serde(default)]
        start_offset: f64,
        #[serde(default)]
        side: TextPathSide,
    },
    Envelope {
        source: Box<Object>,
        kind: crate::core::envelope::EnvelopeKind,
        amount: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub skew_x: f64,
    pub skew_y: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew_x: 0.0,
            skew_y: 0.0,
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> [f64; 6] {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let skew_x_rad = self.skew_x.to_radians();
        let skew_y_rad = self.skew_y.to_radians();

        // Transformation order: Scale & Skew, then Rotate, then Translate
        // (x', y') = R * K * S * (x, y) + T
        // skewed_x = sx * x + sy * y * sin(skew_x)
        // skewed_y = sx * x * sin(skew_y) + sy * y
        // x_rot = cos * skewed_x - sin * skewed_y + tx
        // y_rot = sin * skewed_x + cos * skewed_y + ty
        let k0 = self.scale_x;
        let k1 = self.scale_x * skew_y_rad.sin();
        let k2 = self.scale_y * skew_x_rad.sin();
        let k3 = self.scale_y;

        let m0 = cos * k0 - sin * k1;
        let m1 = sin * k0 + cos * k1;
        let m2 = cos * k2 - sin * k3;
        let m3 = sin * k2 + cos * k3;

        [m0, m1, m2, m3, self.x, self.y]
    }

    pub fn transform_point(&self, px: f64, py: f64) -> (f64, f64) {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let sx = px * self.scale_x;
        let sy = py * self.scale_y;
        let skew_x_rad = self.skew_x.to_radians();
        let skew_y_rad = self.skew_y.to_radians();
        let skewed_x = sx + sy * skew_x_rad.sin();
        let skewed_y = sx * skew_y_rad.sin() + sy;
        (
            cos * skewed_x - sin * skewed_y + self.x,
            sin * skewed_x + cos * skewed_y + self.y,
        )
    }

    pub fn inverse_transform_point(&self, wx: f64, wy: f64) -> (f64, f64) {
        let dx = wx - self.x;
        let dy = wy - self.y;
        let cos = (-self.rotation).cos();
        let sin = (-self.rotation).sin();
        let rx = cos * dx - sin * dy;
        let ry = sin * dx + cos * dy;
        let sx = if self.scale_x.abs() > 1e-6 {
            rx / self.scale_x
        } else {
            0.0
        };
        let sy = if self.scale_y.abs() > 1e-6 {
            ry / self.scale_y
        } else {
            0.0
        };
        (sx, sy)
    }
}

/// Approximate text block metrics: max line width and total height for
/// explicit `\n` line breaks at 1.2em advance (matches canvas + SVG export).
pub fn text_block_size(text: &str, font_size: f64) -> (f64, f64) {
    text_block_size_with_style(text, &TextStyle {
        font_size,
        ..Default::default()
    })
}

/// Measure a text block, honouring explicit line height and word-wrap
/// settings when a `TextStyle` is available.
pub fn text_block_size_with_style(text: &str, style: &TextStyle) -> (f64, f64) {
    let lines = if style.word_wrap {
        if let Some(max_w) = style.max_width {
            compute_wrapped_lines(text, style, max_w)
        } else {
            text.split('\n').map(String::from).collect()
        }
    } else {
        text.split('\n').map(String::from).collect()
    };
    let line_h = style.effective_line_height();
    // Width estimate: sum of per-glyph advances (fullwidth-aware) plus
    // letter-spacing, so measurement agrees with wrapping.
    let width = lines
        .iter()
        .map(|l| {
            l.chars()
                .map(|ch| char_advance_estimate(ch) * style.font_size + style.letter_spacing)
                .sum::<f64>()
        })
        .fold(0.0_f64, f64::max);
    let height = line_h + line_h * (lines.len().saturating_sub(1) as f64);
    (width, height)
}

/// Rough per-glyph advance estimate as a fraction of `font_size`.
///
/// Fullwidth characters (hiragana, katakana, CJK ideographs, hangul,
/// fullwidth forms) advance 1em; halfwidth kana and Latin advance 0.6em.
/// Shared by wrapping and block measurement so both agree.
pub fn char_advance_estimate(ch: char) -> f64 {
    if is_fullwidth(ch) {
        1.0
    } else {
        0.6
    }
}

fn is_fullwidth(ch: char) -> bool {
    matches!(ch,
        '\u{3000}'..='\u{303F}'   // CJK symbols & punctuation (、。〰…)
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{30A0}'..='\u{30FF}' // Katakana
        | '\u{3200}'..='\u{33FF}' // Enclosed CJK & compatibility
        | '\u{3400}'..='\u{4DBF}' // CJK Ext A
        | '\u{4E00}'..='\u{9FFF}' // CJK Unified
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{1100}'..='\u{11FF}' // Hangul Jamo
        | '\u{F900}'..='\u{FAFF}' // CJK Compat Ideographs
        | '\u{FF00}'..='\u{FF60}' // Fullwidth forms…
        | '\u{FFE0}'..='\u{FFE6}' // …and fullwidth symbols
    )
}

/// Characters that must not start a line (行頭禁則: closing brackets,
/// punctuation, prolongation mark, small kana…).
fn kinsoku_cannot_start_line(ch: char) -> bool {
    matches!(ch,
        '、' | '。' | '，' | '．' | '！' | '？' | '!' | '?' | '：' | '；'
        | '）' | '〕' | '］' | '｝' | '〉' | '》' | '」' | '』' | '】' | '\'' | '"' | '’' | '”'
        | '…' | '‥' | '・' | 'ー' | '〜' | '～'
        | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ'
        | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ' | 'ッ' | 'ャ' | 'ュ' | 'ョ'
    )
}

/// Characters that must not end a line (行末禁則: opening brackets).
fn kinsoku_cannot_end_line(ch: char) -> bool {
    matches!(ch,
        '「' | '『' | '（' | '〔' | '［' | '｛' | '〈' | '《' | '【' | '(' | '[' | '{' | '<'
    )
}

/// Laid-out text: drawable lines, how many fit, and the first baseline
/// origin in local coordinates.
pub struct TextLayout {
    pub lines: Vec<String>,
    /// Lines that fit (prefix of `lines`); the rest overflows.
    pub visible: usize,
    /// First-baseline origin (start-anchor x, baseline y) in local coords.
    pub origin: (f64, f64),
}

impl TextLayout {
    pub fn overflow(&self) -> usize {
        self.lines.len().saturating_sub(self.visible)
    }
}

/// Lay out point or area text. Area text wraps to the box width; lines
/// whose baseline falls below the box bottom overflow (still returned —
/// exporters clip them visually but keep them in markup for round-trip).
pub fn layout_text(text: &str, style: &TextStyle, area: Option<TextArea>) -> TextLayout {
    if style.vertical {
        return layout_text_vertical(text, style, area);
    }
    match area {
        None => {
            let lines = if style.word_wrap {
                if let Some(max_w) = style.max_width {
                    compute_wrapped_lines(text, style, max_w)
                } else {
                    text.split('\n').map(String::from).collect()
                }
            } else {
                text.split('\n').map(String::from).collect()
            };
            let visible = lines.len();
            TextLayout {
                lines,
                visible,
                origin: (0.0, 0.0),
            }
        }
        Some(a) => {
            let lines = compute_wrapped_lines(text, style, a.width);
            let line_h = style.effective_line_height().max(1e-6);
            // First baseline at the em-box top + font_size; a line fits
            // while its baseline stays inside the box.
            let capacity =
                ((((a.height - style.font_size) / line_h).floor() as isize) + 1).max(0) as usize;
            let visible = lines.len().min(capacity);
            TextLayout {
                lines,
                visible,
                origin: (a.x, a.y + style.font_size),
            }
        }
    }
}

/// Vertical (縦組み) layout: each source line becomes a column that advances
/// downward; columns advance right→left (vertical-rl). Wrapping uses box
/// *height* as the line budget (chars ≈ height / advance), capacity uses
/// box *width* / column width. Point text (no area) just splits on `\n`.
fn layout_text_vertical(text: &str, style: &TextStyle, area: Option<TextArea>) -> TextLayout {
    let col_advance = style.effective_line_height().max(1e-6); // horizontal distance between columns
    let char_adv = |ch: char| char_advance_estimate(ch) * style.font_size + style.letter_spacing;

    // Split into source lines, optionally wrapping each to the box height.
    let mut columns: Vec<String> = Vec::new();
    match area {
        None => {
            for para in text.split('\n') {
                if style.word_wrap {
                    if let Some(max_h) = style.max_width {
                        // Wrap by character count fitting in max_h.
                        let mut col = String::new();
                        let mut w = 0.0;
                        for ch in para.chars() {
                            let cw = char_adv(ch);
                            if !col.is_empty() && w + cw > max_h {
                                columns.push(std::mem::take(&mut col));
                                w = 0.0;
                            }
                            col.push(ch);
                            w += cw;
                        }
                        columns.push(col);
                    } else {
                        columns.push(para.to_string());
                    }
                } else {
                    columns.push(para.to_string());
                }
            }
        }
        Some(a) => {
            // Available height for one column (em-box top to first baseline budget).
            let max_h = (a.height - style.font_size).max(style.font_size).max(1.0);
            for para in text.split('\n') {
                let mut col = String::new();
                let mut w = 0.0;
                for ch in para.chars() {
                    let cw = char_adv(ch);
                    if !col.is_empty() && w + cw > max_h {
                        columns.push(std::mem::take(&mut col));
                        w = 0.0;
                    }
                    col.push(ch);
                    w += cw;
                }
                columns.push(col);
            }
        }
    }

    // Capacity: how many columns fit in the box width (right edge starts at a.x + a.width).
    let (visible, origin) = match area {
        None => (columns.len(), (0.0, 0.0)),
        Some(a) => {
            let capacity = ((((a.width - style.font_size) / col_advance).floor() as isize) + 1)
                .max(0) as usize;
            // First column's baseline sits near the right edge of the box (vertical-rl).
            let origin_x = a.x + a.width - style.font_size;
            (columns.len().min(capacity), (origin_x, a.y + style.font_size))
        }
    };

    TextLayout {
        lines: columns,
        visible,
        origin,
    }
}

/// Distribute `text` across linked area-text frames (`frames` in link order).
/// Each frame receives as many laid-out lines/columns as it can hold; the
/// remainder overflows into the next frame. Returns one `TextLayout` per frame.
///
/// This is the core of threaded text stories (テキストスレッド). Callers resolve
/// the `next_frame` chain and pass areas in order.
pub fn layout_text_thread(
    text: &str,
    style: &TextStyle,
    frames: &[TextArea],
) -> Vec<TextLayout> {
    if frames.is_empty() {
        return vec![layout_text(text, style, None)];
    }
    // Lay out once as if the first frame owns everything, then slice lines.
    let full = layout_text(text, style, Some(frames[0]));
    let mut out = Vec::with_capacity(frames.len());
    let mut consumed = 0usize;
    for (i, frame) in frames.iter().enumerate() {
        let per_frame = if i + 1 == frames.len() {
            // Last frame keeps whatever is left (may overflow for display).
            full.lines.len().saturating_sub(consumed)
        } else {
            // Capacity of this frame (same formula as layout_text).
            let col_or_line = style.effective_line_height().max(1e-6);
            let capacity = if style.vertical {
                ((((frame.width - style.font_size) / col_or_line).floor() as isize) + 1)
                    .max(0) as usize
            } else {
                ((((frame.height - style.font_size) / col_or_line).floor() as isize) + 1)
                    .max(0) as usize
            };
            capacity.min(full.lines.len().saturating_sub(consumed))
        };
        let end = (consumed + per_frame).min(full.lines.len());
        let slice: Vec<String> = full.lines[consumed..end].to_vec();
        let origin = if style.vertical {
            (frame.x + frame.width - style.font_size, frame.y + style.font_size)
        } else {
            (frame.x, frame.y + style.font_size)
        };
        out.push(TextLayout {
            visible: slice.len(),
            lines: slice,
            origin,
        });
        consumed = end;
    }
    out
}

/// Map every point of a path through a live envelope deform (bbox taken
/// from the path itself). Curves keep their structure with mapped control
/// points (exact on straight runs, approximate under strong bending —
/// same convention as the brush mapper).
pub fn deform_path_data(
    path: &PathData,
    kind: crate::core::envelope::EnvelopeKind,
    amount: f64,
) -> PathData {
    use crate::core::path::PathElement;
    let bb = path.bounding_box().unwrap_or((
        crate::core::path::AnchorPoint::new(0.0, 0.0),
        crate::core::path::AnchorPoint::new(1.0, 1.0),
    ));
    let map = |p: crate::core::path::AnchorPoint| {
        crate::core::envelope::envelope_point(p, bb, kind, amount)
    };
    let mut out = PathData::new();
    out.fill = path.fill.clone();
    out.stroke = path.stroke.clone();
    for el in &path.elements {
        match el {
            PathElement::MoveTo(p) => {
                let q = map(*p);
                out.push_move_to(q.x, q.y);
            }
            PathElement::LineTo(p) => {
                let q = map(*p);
                out.push_line_to(q.x, q.y);
            }
            PathElement::CurveTo(seg) => {
                let s = map(seg.start);
                let c1 = map(seg.control1);
                let c2 = map(seg.control2);
                let e = map(seg.end);
                out.elements.push(PathElement::CurveTo(
                    crate::core::path::BezierSegment {
                        start: s,
                        control1: c1,
                        control2: c2,
                        end: e,
                    },
                ));
            }
            PathElement::ClosePath => out.elements.push(PathElement::ClosePath),
        }
    }
    out
}
/// the last allowed opportunity. Breaks happen at spaces and after CJK
/// characters, subject to Japanese kinsoku rules: a line never starts with
/// a closing mark and never ends with an opening mark (the offending
/// character is pushed to / kept on the adjacent line). Hard breaks (`\n`)
/// always start a new line. Widths use [`char_advance_estimate`] plus
/// `letter_spacing`, so wrapping agrees with block measurement.
pub fn compute_wrapped_lines(text: &str, style: &TextStyle, max_width: f64) -> Vec<String> {
    let unit = |ch: char| char_advance_estimate(ch) * style.font_size + style.letter_spacing;
    let mut result = Vec::new();
    for paragraph in text.split('\n') {
        let chars: Vec<char> = paragraph.chars().collect();
        if chars.is_empty() {
            result.push(String::new());
            continue;
        }
        let mut line_start = 0usize;
        let mut i = 0usize;
        // Byte/char width of the current line for quick slicing.
        let mut line_w = 0.0f64;
        // Last index (exclusive end of line) where a break is allowed.
        let mut last_break: Option<usize> = None;
        while i < chars.len() {
            let ch = chars[i];
            let w = unit(ch);
            // Would this char overflow the line?
            if line_w + w > max_width && i > line_start {
                // Prefer the last allowed break; otherwise force-break
                // before this char.
                let mut end = last_break.unwrap_or(i);
                // Kinsoku: never end a line with an opening bracket — move
                // it (and anything after it on this line) to the next line.
                while end > line_start + 1 && kinsoku_cannot_end_line(chars[end - 1]) {
                    end -= 1;
                }
                // Kinsoku: never start a line with a closing mark — keep it
                // on this line even if it overflows (squeeze emulation,
                // like real Japanese typesetters).
                while end < chars.len() && kinsoku_cannot_start_line(chars[end]) {
                    end += 1;
                }
                // Degenerate width (nothing fits): emit one char to guarantee
                // progress.
                if end <= line_start {
                    end = (line_start + 1).min(chars.len());
                }
                result.push(chars[line_start..end].iter().collect());
                // Skip a single leading space on the new line (Western
                // word-wrap convention); CJK needs no such trimming.
                line_start = end;
                if line_start < chars.len() && chars[line_start] == ' ' {
                    line_start += 1;
                }
                i = line_start;
                line_w = 0.0;
                last_break = None;
                continue;
            }
            line_w += w;
            // A break is allowed *after* this char when the next char may
            // legally start a line and this char may legally end one.
            let next_ok = i + 1 >= chars.len()
                || !kinsoku_cannot_start_line(chars[i + 1]);
            if (ch == ' ' || is_fullwidth(ch)) && !kinsoku_cannot_end_line(ch) && next_ok {
                last_break = Some(i + 1);
            }
            i += 1;
        }
        result.push(chars[line_start..].iter().collect());
    }
    result
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub id: String,
    pub name: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub fill: Option<FillStyle>,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<DropShadow>,
    pub glow: Option<GlowEffect>,
    /// Non-destructive Illustrator-style appearance effects (stackable blur,
    /// color adjustments, ...). Missing in older project files, so it must
    /// default or every legacy `.amata`/JSON document fails to deserialize.
    #[serde(default)]
    pub appearance: AppearanceStack,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    #[serde(default)]
    pub width_profile: Option<WidthProfile>,
    /// Figma-style auto layout when this object is a group.
    #[serde(default)]
    pub auto_layout: Option<crate::core::auto_layout::AutoLayout>,
    pub visible: bool,
    pub locked: bool,
}

impl Object {
    pub fn new_path(name: &str, path: PathData) -> Self {
        let fill = path.fill.clone();
        let stroke = path.stroke.clone();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Path(path),
            transform: Transform::default(),
            fill,
            stroke,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_rect(name: &str, x: f64, y: f64, w: f64, h: f64, corner_radius: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Rectangle {
                width: w,
                height: h,
                corner_radius,
            },
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_ellipse(name: &str, cx: f64, cy: f64, rx: f64, ry: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Ellipse { rx, ry },
            transform: Transform {
                x: cx,
                y: cy,
                ..Default::default()
            },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_star(
        name: &str,
        cx: f64,
        cy: f64,
        points: usize,
        inner_radius: f64,
        outer_radius: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Star {
                points,
                inner_radius,
                outer_radius,
            },
            transform: Transform {
                x: cx,
                y: cy,
                ..Default::default()
            },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_polygon(name: &str, cx: f64, cy: f64, sides: usize, radius: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Polygon { sides, radius },
            transform: Transform {
                x: cx,
                y: cy,
                ..Default::default()
            },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_line(name: &str, x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Line {
                x2: x2 - x1,
                y2: y2 - y1,
            },
            transform: Transform {
                x: x1,
                y: y1,
                ..Default::default()
            },
            fill: None,
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_text(name: &str, text: &str, x: f64, y: f64, font_size: f64) -> Self {
        Self::new_text_with_style(
            name,
            text,
            x,
            y,
            TextStyle::new("Inter, sans-serif", font_size),
        )
    }

    pub fn new_text_with_style(name: &str, text: &str, x: f64, y: f64, style: TextStyle) -> Self {
        let font_size = style.font_size;
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Text {
                text: text.to_string(),
                font_size,
                style,
                area: None,
                next_frame: None,
            },
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
            fill: Some(FillStyle::default()),
            stroke: None,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_text_on_path(name: &str, text: &str, path: PathData) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::TextOnPath {
                text: text.to_string(),
                style: TextStyle::new("Inter, sans-serif", 24.0),
                path,
                source_path_id: None,
                start_offset: 0.0,
                side: TextPathSide::Top,
            },
            transform: Transform::default(),
            fill: Some(FillStyle::default()),
            stroke: None,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_group(name: &str, objects: Vec<Object>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Group(objects),
            transform: Transform::default(),
            fill: None,
            stroke: None,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_use(
        name: &str,
        href: &str,
        x: f64,
        y: f64,
        width: Option<f64>,
        height: Option<f64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Use {
                href: href.to_string(),
                width,
                height,
            },
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
            fill: None,
            stroke: None,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_image(
        name: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        png_bytes: Vec<u8>,
    ) -> Self {        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Image {
                width,
                height,
                png_bytes,
            },
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
            fill: None,
            stroke: None,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_pixel_art(
        name: &str,
        x: f64,
        y: f64,
        pixels: crate::core::pixel::PixelArt,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::PixelArt(pixels),
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
            fill: None,
            stroke: None,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_mesh(name: &str, x: f64, y: f64, mesh: crate::core::gradient_mesh::MeshGradient) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::GradientMesh(mesh),
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
            fill: None,
            stroke: None,
            shadow: None,
            glow: None,
            appearance: AppearanceStack::default(),
            opacity: 1.0,
            width_profile: None,
            auto_layout: None,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    /// Wrap a vector shape/path in a live envelope. The source is baked to
    /// a Path with identity transform (so the deform space stays sane);
    /// release restores a plain path. Returns `None` for text, images and
    /// other non-vector sources.
    pub fn wrap_envelope(
        name: &str,
        source: &Object,
        kind: crate::core::envelope::EnvelopeKind,
        amount: f64,
    ) -> Option<Self> {
        match &source.object_type {
            ObjectType::Path(_)
            | ObjectType::Rectangle { .. }
            | ObjectType::Ellipse { .. }
            | ObjectType::Star { .. }
            | ObjectType::Polygon { .. }
            | ObjectType::Line { .. } => {}
            _ => return None,
        }
        let mut path = source.to_path_data();
        path.transform(&source.transform.matrix());
        path.fill = source.fill.clone();
        path.stroke = source.stroke.clone();
        // Give the deform something to bend: sparse edges are subdivided
        // shape-exactly (release stays lossless).
        path = crate::core::envelope::subdivide_path(&path, 2);
        let mut baked = Self::new_path(&format!("{} (Source)", source.name), path);
        baked.transform = Transform::default();
        baked.fill = None;
        baked.stroke = None;
        Some(Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Envelope {
                source: Box::new(baked),
                kind,
                amount: amount.clamp(-1.0, 1.0),
            },
            transform: Transform::default(),
            fill: source.fill.clone(),
            stroke: source.stroke.clone(),
            shadow: source.shadow.clone(),
            glow: source.glow.clone(),
            appearance: AppearanceStack::default(),
            opacity: source.opacity,
            width_profile: None,
            auto_layout: None,
            blend_mode: source.blend_mode,
            visible: true,
            locked: false,
        })
    }

    /// Render-ready proxy: the deformed source as a plain Path carrying
    /// this object's paint/transform. Renderers recurse into it instead of
    /// duplicating path-drawing code.
    pub fn envelope_proxy(&self) -> Option<Object> {
        if let ObjectType::Envelope { .. } = &self.object_type {
            let mut proxy = self.clone();
            proxy.object_type = ObjectType::Path(self.to_path_data());
            Some(proxy)
        } else {
            None
        }
    }

    /// Combine multiple objects into a single Compound Path (holes are created where subpaths overlap using EvenOdd rule)
    pub fn make_compound_path(objects: &[Object]) -> Option<Self> {
        if objects.is_empty() {
            return None;
        }

        let base = &objects[0];
        let mut combined_path = PathData::new();
        combined_path.fill = base.fill.clone().or_else(|| Some(FillStyle::default()));
        if let Some(ref mut fill) = combined_path.fill {
            fill.rule = crate::core::path::FillRule::EvenOdd;
        }
        combined_path.stroke = base.stroke.clone();

        for obj in objects {
            let mut p = obj.to_path_data();
            p.transform(&obj.transform.matrix());
            combined_path.elements.extend(p.elements);
        }

        let mut compound = Self::new_path("Compound Path", combined_path);
        compound.shadow = base.shadow.clone();
        compound.glow = base.glow.clone();
        compound.opacity = base.opacity;
        compound.blend_mode = base.blend_mode;
        Some(compound)
    }

    /// Release a Compound Path into its component independent subpath objects
    pub fn release_compound_path(&self) -> Vec<Self> {
        if let ObjectType::Path(ref path) = self.object_type {
            let subpaths = path.to_subpaths(16);
            if subpaths.len() <= 1 {
                return vec![self.clone()];
            }

            let mut released = Vec::new();
            for (idx, sp) in subpaths.into_iter().enumerate() {
                let mut p = PathData::from_polygon_points(&sp, true);
                p.fill = self.fill.clone();
                p.stroke = self.stroke.clone();
                let mut obj = Self::new_path(&format!("{}_part_{}", self.name, idx + 1), p);
                obj.transform = self.transform.clone();
                obj.shadow = self.shadow.clone();
                obj.glow = self.glow.clone();
                obj.opacity = self.opacity;
                obj.blend_mode = self.blend_mode;
                released.push(obj);
            }
            released
        } else {
            vec![self.clone()]
        }
    }

    /// Converts this object to its canonical local PathData representation
    pub fn to_path_data(&self) -> PathData {
        match &self.object_type {
            ObjectType::Path(path) => path.clone(),
            ObjectType::Rectangle {
                width,
                height,
                corner_radius,
            } => PathData::from_rect(0.0, 0.0, *width, *height, *corner_radius),
            ObjectType::Ellipse { rx, ry } => PathData::from_ellipse(0.0, 0.0, *rx, *ry),
            ObjectType::Star {
                points,
                inner_radius,
                outer_radius,
            } => PathData::from_star(*points, *inner_radius, *outer_radius, 0.0, 0.0),
            ObjectType::Polygon { sides, radius } => {
                PathData::from_polygon(*sides, *radius, 0.0, 0.0)
            }
            ObjectType::Line { x2, y2 } => PathData::from_line(0.0, 0.0, *x2, *y2),
            ObjectType::Text {
                text, font_size, style, area, ..
            } => {
                // Area text selects by its box; point text by the measured
                // block (first baseline at y=0, 1.2em line advance).
                if let Some(a) = area {
                    return PathData::from_rect(a.x, a.y, a.width, a.height, 0.0);
                }
                let (width, height) = text_block_size_with_style(text, style);
                if style.vertical {
                    // Bounding box for vertical text: height along X, width along Y
                    // is approximate (char_advance uses horizontal widths).
                    return PathData::from_rect(0.0, -font_size, height.max(width), width, 0.0);
                }
                PathData::from_rect(0.0, -font_size, width, height, 0.0)
            }
            ObjectType::Group(children) => {
                let mut combined = PathData::new();
                for child in children {
                    let mut child_path = child.to_path_data();
                    child_path.transform(&child.transform.matrix());
                    combined.elements.extend(child_path.elements);
                }
                combined
            }
            ObjectType::ClippingMask { children } => {
                let mut combined = PathData::new();
                for child in children {
                    let mut child_path = child.to_path_data();
                    child_path.transform(&child.transform.matrix());
                    combined.elements.extend(child_path.elements);
                }
                combined
            }
            ObjectType::Use { width, height, .. } => {
                let w = width.unwrap_or(100.0);
                let h = height.unwrap_or(100.0);
                PathData::from_rect(0.0, 0.0, w, h, 0.0)
            }
            ObjectType::Image { width, height, .. } => {
                PathData::from_rect(0.0, 0.0, *width, *height, 0.0)
            }
            ObjectType::PixelArt(p) => {
                PathData::from_rect(0.0, 0.0, p.width as f64, p.height as f64, 0.0)
            }
            ObjectType::GradientMesh(m) => match m.node_bbox() {
                Some((mn, mx)) => PathData::from_rect(mn.x, mn.y, mx.x - mn.x, mx.y - mn.y, 0.0),
                None => PathData::new(),
            },
            ObjectType::TextOnPath {
                text,
                style,
                path: base,
                start_offset,
                side,
                ..
            } => crate::core::text_path::text_on_path_outlines(
                base,
                text,
                style,
                *start_offset,
                *side,
            ),
            ObjectType::Envelope { source, kind, amount } => {
                deform_path_data(&source.to_path_data(), *kind, amount.clamp(-1.0, 1.0))
            },
        }
    }

    pub fn world_path(&self) -> Option<PathData> {
        let mut path = self.to_path_data();
        path.transform(&self.transform.matrix());
        path.fill = self.fill.clone();
        path.stroke = self.stroke.clone();
        Some(path)
    }

    /// Convert object to world polygon points for hit testing and Boolean operations
    pub fn to_world_polygon(&self) -> Vec<AnchorPoint> {
        let mut path = self.to_path_data();
        path.transform(&self.transform.matrix());
        path.to_polygon(16)
    }

    pub fn hit_test(&self, px: f64, py: f64) -> bool {
        let (lx, ly) = self.transform.inverse_transform_point(px, py);

        match &self.object_type {
            ObjectType::Path(path) => {
                let subpaths = path.to_subpaths(8);
                let even_odd = path
                    .fill
                    .as_ref()
                    .map(|f| matches!(f.rule, crate::core::path::FillRule::EvenOdd))
                    .unwrap_or(true);
                crate::core::geometry::point_in_subpaths(lx, ly, &subpaths, even_odd)
            }
            ObjectType::Rectangle { width, height, .. } => {
                lx >= 0.0 && lx <= *width && ly >= 0.0 && ly <= *height
            }
            ObjectType::Ellipse { rx, ry } => {
                if *rx <= 0.0 || *ry <= 0.0 {
                    return false;
                }
                let dx = lx / rx;
                let dy = ly / ry;
                dx * dx + dy * dy <= 1.0
            }
            ObjectType::Star { .. } | ObjectType::Polygon { .. } => {
                let poly = self.to_path_data().to_polygon(8);
                crate::core::geometry::point_in_polygon(lx, ly, &poly)
            }
            ObjectType::Line { x2, y2 } => {
                let dist = crate::core::geometry::distance_to_segment(
                    AnchorPoint::new(lx, ly),
                    AnchorPoint::new(0.0, 0.0),
                    AnchorPoint::new(*x2, *y2),
                );
                let stroke_w = self.stroke.as_ref().map(|s| s.width).unwrap_or(2.0);
                dist <= (stroke_w / 2.0).max(4.0)
            }
            ObjectType::Text {
                text, font_size, style, area, ..
            } => {
                if let Some(a) = area {
                    return lx >= a.x && lx <= a.x + a.width && ly >= a.y && ly <= a.y + a.height;
                }
                let (width, height) = text_block_size_with_style(text, style);
                if style.vertical {
                    let h = height.max(width);
                    let w = width;
                    return lx >= 0.0 && lx <= h && ly >= -font_size && ly <= -font_size + w;
                }
                lx >= 0.0 && lx <= width && ly >= -font_size && ly <= -font_size + height
            }
            ObjectType::Group(children) => children.iter().any(|c| c.hit_test(lx, ly)),
            ObjectType::ClippingMask { children } => children.iter().any(|c| c.hit_test(lx, ly)),
            ObjectType::Use { width, height, .. } => {
                let w = width.unwrap_or(100.0);
                let h = height.unwrap_or(100.0);
                lx >= 0.0 && lx <= w && ly >= 0.0 && ly <= h
            }
            ObjectType::Image { width, height, .. } => {
                lx >= 0.0 && lx <= *width && ly >= 0.0 && ly <= *height
            }
            ObjectType::PixelArt(p) => {
                lx >= 0.0 && lx <= p.width as f64 && ly >= 0.0 && ly <= p.height as f64
            }
            ObjectType::GradientMesh(m) => match m.node_bbox() {
                Some((mn, mx)) => lx >= mn.x && lx <= mx.x && ly >= mn.y && ly <= mx.y,
                None => false,
            },
            ObjectType::TextOnPath { .. } => {
                let outlines = self.to_path_data();
                let subpaths = outlines.to_subpaths(8);
                // Glyph counters (holes in A/B/…) require even-odd.
                crate::core::geometry::point_in_subpaths(lx, ly, &subpaths, true)
            },
            ObjectType::Envelope { .. } => {
                let poly = self.to_path_data().to_polygon(8);
                crate::core::geometry::point_in_polygon(lx, ly, &poly)
            },
        }
    }

    pub fn bounding_box(&self) -> Option<(AnchorPoint, AnchorPoint)> {
        let poly = self.to_world_polygon();
        if poly.is_empty() {
            return None;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for p in &poly {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        if min_x <= max_x && min_y <= max_y {
            Some((
                AnchorPoint::new(min_x, min_y),
                AnchorPoint::new(max_x, max_y),
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BlendMode;
    use crate::core::blend::{blend_colors, BlendMode as CoreBlendMode};

    const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

    fn assert_close(actual: [f32; 4], expected: [f32; 4]) {
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!(
                (a - e).abs() < 1e-6,
                "expected {expected:?}, got {actual:?}"
            );
        }
    }

    /// The document's serialized mode list must cover exactly the separable
    /// modes `core::blend` already implements under the same names.
    #[test]
    fn to_blend_covers_every_document_mode() {
        let expected = [
            (BlendMode::Normal, CoreBlendMode::Normal),
            (BlendMode::Multiply, CoreBlendMode::Multiply),
            (BlendMode::Screen, CoreBlendMode::Screen),
            (BlendMode::Overlay, CoreBlendMode::Overlay),
            (BlendMode::Darken, CoreBlendMode::Darken),
            (BlendMode::Lighten, CoreBlendMode::Lighten),
            (BlendMode::ColorDodge, CoreBlendMode::ColorDodge),
            (BlendMode::ColorBurn, CoreBlendMode::ColorBurn),
            (BlendMode::HardLight, CoreBlendMode::HardLight),
            (BlendMode::SoftLight, CoreBlendMode::SoftLight),
            (BlendMode::Difference, CoreBlendMode::Difference),
            (BlendMode::Exclusion, CoreBlendMode::Exclusion),
            (BlendMode::Hue, CoreBlendMode::Hue),
            (BlendMode::Saturation, CoreBlendMode::Saturation),
            (BlendMode::Color, CoreBlendMode::Color),
            (BlendMode::Luminosity, CoreBlendMode::Luminosity),
        ];
        assert_eq!(BlendMode::all().len(), expected.len());
        for (doc, core) in expected {
            assert_eq!(doc.to_blend(), core, "{doc:?} mapped wrongly");
        }
    }

    /// The canvas previews a blended fill against the white artboard, so
    /// Multiply must be a no-op there while Screen blows out to white.
    #[test]
    fn blend_preview_against_white_artboard() {
        assert_close(
            blend_colors(RED, WHITE, BlendMode::Multiply.to_blend()),
            RED,
        );
        assert_close(
            blend_colors(RED, WHITE, BlendMode::Screen.to_blend()),
            WHITE,
        );
        // Normal keeps the source untouched, alpha included.
        assert_close(blend_colors(RED, WHITE, BlendMode::Normal.to_blend()), RED);
    }
}
