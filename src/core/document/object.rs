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
    /// Column count for area text (1 = no columns). Lines flow down the
    /// first column, then continue at the top of the next.
    #[serde(default = "default_cols")]
    pub cols: u32,
    /// Gutter between columns, in document units.
    #[serde(default = "default_gutter")]
    pub gutter: f64,
}

fn default_cols() -> u32 {
    1
}

fn default_gutter() -> f64 {
    12.0
}

impl TextArea {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(1.0),
            height: height.max(1.0),
            cols: 1,
            gutter: 12.0,
        }
    }

    /// Copy with hostile values scrubbed: hand-edited files may carry
    /// NaN/Inf geometry (the constructor only clamps w/h, and serde bypasses
    /// it entirely). NaN origins poison every export stream downstream.
    pub fn sanitized(&self) -> TextArea {
        fn finite_or(v: f64, fallback: f64) -> f64 {
            if v.is_finite() {
                v
            } else {
                fallback
            }
        }
        TextArea {
            x: finite_or(self.x, 0.0),
            y: finite_or(self.y, 0.0),
            width: finite_or(self.width, 1.0).max(1.0),
            height: finite_or(self.height, 1.0).max(1.0),
            cols: self.cols.max(1),
            gutter: finite_or(self.gutter, 12.0).max(0.0),
        }
    }

    /// Per-column widths for `cols` columns across `width`.
    pub fn column_widths(&self) -> Vec<f64> {
        let n = self.cols.max(1) as usize;
        if n == 1 {
            return vec![self.width];
        }
        let total_gutter = self.gutter.max(0.0) * (n as f64 - 1.0);
        let w = ((self.width - total_gutter) / n as f64).max(1.0);
        vec![w; n]
    }

    /// Absolute x of each column's left edge.
    pub fn column_origins(&self) -> Vec<f64> {
        let mut xs = Vec::new();
        let mut x = self.x;
        for w in self.column_widths() {
            xs.push(x);
            x += w + self.gutter.max(0.0);
        }
        xs
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
    /// Auto-insert spacing at Japanese/Latin boundaries (和欧間).
    #[serde(default = "default_auto_spacing")]
    pub auto_spacing: bool,
    /// Size of the [`TextStyle::auto_spacing`] gap, in em
    /// (0.25 = the classic 1/4em; 0.5 is the wider 1/2em some
    /// houses use).
    #[serde(default = "default_auto_spacing_em")]
    pub auto_spacing_em: f32,
    /// Enable GSUB `liga`/`dlig`/`clig`/`rlig` ligature substitution
    /// when the face provides them (outline path only; SVG keeps raw text).
    #[serde(default = "default_ligatures")]
    pub ligatures: bool,
    /// Explicit OpenType feature overrides (4-char tag + on/off), applied
    /// after the `ligatures` toggle. Empty = shaper defaults. Used for the
    /// Japanese typography features the shaper leaves off by default
    /// (`palt`, `halt`, `vert`, `vrt2`, `ruby`, `kern`, `vkrn`, …).
    #[serde(default)]
    pub ot_features: Vec<OtFeature>,
    /// List marker style (DTP): prefixes each paragraph and hangs wrapped
    /// continuation lines by the marker width.
    #[serde(default)]
    pub list: ListStyle,
    /// ぶら下げ (hanging punctuation): let a line's trailing 閉じ約物
    /// (、。・」）…) overrun the wrap width by its *ink*, instead of the
    /// whole em box. Japanese typesetting convention; Illustrator exposes
    /// the same switch in its Japanese typesography settings.
    #[serde(default = "default_burasage")]
    pub burasage: bool,
}

/// One OpenType feature override: a 4-char tag (`palt`, `vert`, `ruby`,
/// `jp90`, …) and whether it is forced on or off. Entries with an
/// unknown/oversized tag are ignored at shaping time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtFeature {
    pub tag: String,
    pub on: bool,
}

impl OtFeature {
    pub fn new(tag: impl Into<String>, on: bool) -> Self {
        Self {
            tag: tag.into(),
            on,
        }
    }

    /// The tag as exactly 4 ASCII bytes, or `None` when malformed.
    pub fn tag_bytes(&self) -> Option<[u8; 4]> {
        let bytes = self.tag.as_bytes();
        if bytes.len() != 4 || !bytes.iter().all(|b| b.is_ascii_alphanumeric()) {
            return None;
        }
        let mut out = [0u8; 4];
        out.copy_from_slice(bytes);
        Some(out)
    }
}

/// Paragraph list marker style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ListStyle {
    /// No marker.
    #[default]
    None,
    /// Bulleted (`• `).
    Bullet,
    /// Auto-numbered (`1. `, restarting per text object).
    Numbered,
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
            auto_spacing: true,
            auto_spacing_em: 0.25,
            ligatures: true,
            ot_features: Vec::new(),
            list: ListStyle::None,
            burasage: true,
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
            auto_spacing: true,
            auto_spacing_em: 0.25,
            ligatures: true,
            ot_features: Vec::new(),
            list: ListStyle::None,
            burasage: true,
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
    /// Non-finite values are ignored: a NaN/Inf coordinate would reach the
    /// HarfBuzz instance and ttf-parser as NaN advances (poisoning every
    /// outline bbox downstream).
    pub fn set_variation(&mut self, axis: impl Into<String>, value: f64) {
        if !value.is_finite() {
            return;
        }
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

    /// Current explicit OpenType override for a 4-char tag, if any.
    pub fn ot_feature_state(&self, tag: &str) -> Option<bool> {
        self.ot_features
            .iter()
            .find(|f| f.tag.eq_ignore_ascii_case(tag))
            .map(|f| f.on)
    }

    /// True when the `halt` (Alternate Half Widths) feature is forced on.
    /// Default is off, matching HarfBuzz.
    pub fn halt_on(&self) -> bool {
        self.ot_feature_state("halt") == Some(true)
    }

    /// Force a 4-char OpenType feature on or off (undo is the caller's
    /// job). Malformed tags are ignored.
    pub fn set_ot_feature(&mut self, tag: impl Into<String>, on: bool) {
        let feature = OtFeature::new(tag, on);
        if feature.tag_bytes().is_none() {
            return;
        }
        if let Some(entry) = self
            .ot_features
            .iter_mut()
            .find(|f| f.tag.eq_ignore_ascii_case(&feature.tag))
        {
            entry.on = on;
        } else {
            self.ot_features.push(feature);
        }
    }

    /// Drop an explicit override, returning to the shaper default.
    pub fn clear_ot_feature(&mut self, tag: &str) {
        self.ot_features
            .retain(|f| !f.tag.eq_ignore_ascii_case(tag));
    }

    /// Effective feature pairs for the shaper: the legacy `ligatures`
    /// toggle first (it predates explicit overrides), then `ot_features`
    /// in order so a later entry wins. Tags are 4 bytes; values are
    /// HarfBuzz-style booleans (1 = on, 0 = off).
    pub fn ot_feature_pairs(&self) -> Vec<([u8; 4], u32)> {
        let mut out: Vec<([u8; 4], u32)> = Vec::new();
        let liga_on = if self.ligatures { 1 } else { 0 };
        for tag in [b"liga", b"dlig", b"clig", b"rlig"] {
            out.push((*tag, liga_on));
        }
        for feature in &self.ot_features {
            let Some(tag) = feature.tag_bytes() else {
                continue;
            };
            match out.iter_mut().find(|(t, _)| *t == tag) {
                Some(slot) => slot.1 = u32::from(feature.on),
                None => out.push((tag, u32::from(feature.on))),
            }
        }
        out
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
                // "NaN"/"inf" parse successfully — never store them (see
                // `set_variation` for why non-finite coordinates poison).
                if value.is_finite() {
                    out.push(VariationSetting::new(tag, value));
                }
            }
        }
        out
    }

    /// Effective line height in document units, hardened for hostile input:
    /// deserialized documents may carry NaN/Inf/negative multipliers or sizes
    /// (the UI clamps its own widgets, JSON does not). Falls back to the
    /// 1.2em default so layout math never goes NaN (which would poison
    /// bounding boxes and export streams downstream).
    pub fn effective_line_height(&self) -> f64 {
        let mult = match self.line_height {
            Some(v) if v.is_finite() && v > 0.0 => v,
            Some(_) => 1.2,
            None => 1.2,
        };
        mult * self.effective_font_size()
    }

    /// Font size with hostile values (NaN/Inf/<=0 from hand-edited files)
    /// replaced by the default, so advances and scales stay finite.
    pub fn effective_font_size(&self) -> f64 {
        if self.font_size.is_finite() && self.font_size > 0.0 {
            self.font_size
        } else {
            default_font_size()
        }
    }

    /// Letter spacing with non-finite hostile values dropped to zero, so
    /// advances never go NaN (a NaN advance poisons outline bboxes and the
    /// PDF content stream alike).
    pub fn effective_letter_spacing(&self) -> f64 {
        if self.letter_spacing.is_finite() {
            self.letter_spacing
        } else {
            0.0
        }
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
    text_block_size_with_style(
        text,
        &TextStyle {
            font_size,
            ..Default::default()
        },
    )
}

/// Measure a text block, honouring explicit line height and word-wrap
/// settings when a `TextStyle` is available.
pub fn text_block_size_with_style(text: &str, style: &TextStyle) -> (f64, f64) {
    let normalized = normalize_text(text);
    let text = normalized.as_ref();
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
        .map(|l| text_advance_estimate(l, style))
        .fold(0.0_f64, f64::max);
    let height = line_h + line_h * (lines.len().saturating_sub(1) as f64);
    (width, height)
}

/// Normalize line/paragraph separators to `\n` before layout, measurement
/// or shaping. Pasted Windows text (`\r\n`), legacy Mac (`\r`) and
/// U+2028/2029 paragraph separators otherwise leave a stray control char at
/// the end of a line: no font maps it, so the whole run degrades to
/// `.notdef`/mock-block fallback (visible tofu for otherwise fine text).
pub fn normalize_text(text: &str) -> std::borrow::Cow<'_, str> {
    if !text
        .chars()
        .any(|c| c == '\r' || c == '\u{2028}' || c == '\u{2029}')
    {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                // Collapse CRLF to a single break.
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\u{2028}' | '\u{2029}' => out.push('\n'),
            _ => out.push(c),
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Serde default for [`TextStyle::auto_spacing`] (on, matching the
/// constructor defaults).
fn default_auto_spacing() -> bool {
    true
}

/// Serde default for [`TextStyle::auto_spacing_em`] (1/4em).
fn default_auto_spacing_em() -> f32 {
    0.25
}

/// Serde default for [`TextStyle::ligatures`]: documents written before
/// the field existed keep the constructor's on-state.
fn default_ligatures() -> bool {
    true
}

/// Serde default for [`TextStyle::burasage`].
fn default_burasage() -> bool {
    true
}

/// How far a hanging char may overrun the wrap width, in em:
/// half-width for punctuation (、。・…), full-width for closing brackets
/// (」）〕…), zero for everything else. With `burasage` off the whole em
/// box overruns instead (the classic 追い込み behaviour).
pub fn burasage_hang(ch: char) -> f64 {
    if matches!(
        ch,
        '、' | '。'
            | '，'
            | '．'
            | '・'
            | '：'
            | '；'
            | '！'
            | '？'
            | '･'
            | '…'
            | '‥'
            | '—'
            | '―'
            | '〜'
            | '～'
    ) {
        0.5
    } else if matches!(
        ch,
        '」' | '』'
            | '）'
            | '〕'
            | '］'
            | '｝'
            | '〉'
            | '》'
            | '】'
            | '｣'
            | '’'
            | '”'
            | '"'
            | '\''
            | ')'
            | ']'
            | '}'
    ) {
        1.0
    } else {
        0.0
    }
}

/// True when two adjacent significant characters straddle the
/// 和文/欧文 boundary (fullwidth Japanese punctuation, kana, CJK, against
/// an ASCII letter or digit).
pub fn is_ja_latin_boundary(a: char, b: char) -> bool {
    let a_ja = is_fullwidth(a) && !is_zero_width(a);
    let b_ja = is_fullwidth(b) && !is_zero_width(b);
    let a_lat = a.is_ascii_alphanumeric();
    let b_lat = b.is_ascii_alphanumeric();
    (a_ja && b_lat) || (a_lat && b_ja)
}

/// Fullwidth detection (vertical stacking: these stand upright).
pub fn is_fullwidth_char(ch: char) -> bool {
    is_fullwidth(ch)
}

/// True when the text contains at least one 和/欧 boundary pair.
pub fn has_ja_latin_boundary(text: &str) -> bool {
    let mut prev: Option<char> = None;
    for ch in text.chars() {
        if !is_zero_width(ch) {
            if let Some(p) = prev {
                if is_ja_latin_boundary(p, ch) {
                    return true;
                }
            }
            prev = Some(ch);
        }
    }
    false
}

/// Extra advance inserted at a 和欧 boundary: `em` × font size.
pub fn ja_latin_gap_em(font_size: f64, em: f64) -> f64 {
    font_size * em
}

/// Extra advance inserted at a 和欧 boundary: ~1/4em.
pub fn ja_latin_gap(font_size: f64) -> f64 {
    ja_latin_gap_em(font_size, 0.25)
}

/// Advance of every glyph in `text` plus letter-spacing plus auto-spacing
/// gaps at 和欧 boundaries. Shared by wrapping, measuring and the
/// outline shaping loops so they all agree.
pub fn text_advance_estimate(text: &str, style: &TextStyle) -> f64 {
    let font_size = style.effective_font_size();
    let letter_spacing = style.effective_letter_spacing();
    let mut w = 0.0;
    let mut prev: Option<char> = None;
    for ch in text.chars() {
        w += char_advance_styled(ch, style) * font_size + letter_spacing;
        if style.auto_spacing {
            if let Some(p) = prev {
                if is_ja_latin_boundary(p, ch) {
                    w += ja_latin_gap_em(font_size, style.auto_spacing_em as f64);
                }
            }
        }
        if !is_zero_width(ch) {
            prev = Some(ch);
        }
    }
    w
}

/// Split `text` into runs between 和欧 boundaries. Each entry is
/// `(segment, gap_before)`: `gap_before` is true for the segment that
/// starts after a Japanese/Latin boundary (i.e. an explicit gap of
/// `ja_latin_gap` belongs before it).
pub fn split_ja_latin_segments(text: &str, style: &TextStyle) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut prev: Option<char> = None;
    for ch in text.chars() {
        let gap = style.auto_spacing && prev.map(|p| is_ja_latin_boundary(p, ch)).unwrap_or(false);
        if gap {
            out.push((ch.to_string(), true));
        } else if out.is_empty() {
            out.push((ch.to_string(), false));
        } else {
            out.last_mut().unwrap().0.push(ch);
        }
        if !is_zero_width(ch) {
            prev = Some(ch);
        }
    }
    out
}

/// Advance of `ch` under `style`: [`char_advance_estimate`] plus the
/// `halt` (Alternate Half Widths) halving when the style forces it on.
/// Every estimate-driven consumer (wrapping, measurement, canvas
/// positioning) uses this so they agree with each other — and with the
/// HarfBuzz outline path, which applies the font's own `halt` forms.
pub fn char_advance_styled(ch: char, style: &TextStyle) -> f64 {
    let base = char_advance_estimate(ch);
    if base > 0.0 && style.halt_on() && is_halt_char(ch) {
        return base / 2.0;
    }
    base
}

/// Rough per-glyph advance estimate as a fraction of `font_size`.
///
/// Fullwidth characters (hiragana, katakana, CJK ideographs, hangul,
/// fullwidth forms, emoji/symbols that Japanese fonts set fullwidth)
/// advance 1em; combining marks, variation selectors (IVS/VS1–16), ZWJ/ZWNJ
/// and ZWNBSP advance 0 (they ride on the previous glyph — counting 0.6em
/// for each VS16/ZWJ made every emoji run wrap far too early);
/// everything else advances 0.6em.
/// Shared by wrapping and block measurement so both agree.
pub fn char_advance_estimate(ch: char) -> f64 {
    if is_zero_width(ch) {
        0.0
    } else if is_fullwidth(ch) {
        1.0
    } else {
        0.6
    }
}

/// Zero-advance format characters: variation selectors (U+FE00–FE0F,
/// incl. text/emoji presentation VS15/VS16), IVS selectors (U+E0100–E01EF),
/// joiners and combining marks. They modify the previous glyph instead of
/// advancing the pen.
fn is_zero_width(ch: char) -> bool {
    matches!(ch,
        '\u{200C}' | '\u{200D}' | '\u{FEFF}' // ZWNJ, ZWJ, ZWNBSP
        | '\u{0300}'..='\u{036F}' // combining diacriticals
        | '\u{FE00}'..='\u{FE0F}' // variation selectors VS1–VS16
        | '\u{E0100}'..='\u{E01EF}' // IVS selectors
    )
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
        | '\u{2500}'..='\u{257F}' // Box drawing (fullwidth in JP fonts)
        | '\u{25A0}'..='\u{25FF}' // Geometric shapes (■●▲…)
        | '\u{2600}'..='\u{27BF}' // Misc symbols, dingbats, enclosed forms
        | '\u{2B00}'..='\u{2BFF}' // Misc symbols and arrows
        | '\u{1F000}'..='\u{1FAFF}' // Emoji & pictographs (fullwidth advance)
    )
}

/// Characters that must not start a line (行頭禁則: closing brackets,
/// punctuation, prolongation mark, iteration marks, small kana…).
fn kinsoku_cannot_start_line(ch: char) -> bool {
    matches!(
        ch,
        '、' | '。' | '，' | '．' | '！' | '？' | '!' | '?' | '：' | '；' | '＂' | '＇'
        | '）' | '〕' | '］' | '｝' | '〉' | '》' | '」' | '』' | '】' | '｣' | '\'' | '"' | '’' | '”'
        | '…' | '‥' | '・' | 'ー' | 'ｰ' | '—' | '―' | '〜' | '～'
        | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ' | 'ゕ' | 'ゖ'
        | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ' | 'ッ' | 'ャ' | 'ュ' | 'ョ' | 'ヮ'
        | 'ｧ' | 'ｨ' | 'ｩ' | 'ｪ' | 'ｫ' | 'ｯ' | 'ｬ' | 'ｭ' | 'ｮ' // halfwidth small kana
        | 'ゝ' | 'ゞ' | 'ヽ' | 'ヾ' // iteration marks
    )
}

/// Characters that must not end a line (行末禁則: opening brackets).
fn kinsoku_cannot_end_line(ch: char) -> bool {
    matches!(
        ch,
        '「' | '『'
            | '（'
            | '〔'
            | '［'
            | '｛'
            | '〈'
            | '《'
            | '【'
            | '｢'
            | '('
            | '['
            | '{'
            | '<'
            | '‘'
            | '“'
            | '«'
            | '‹'
    )
}

/// 縦中横 (tate-chū-yoko) unit: a short ASCII-digit run inside vertical
/// text that is typeset horizontally inside one em cell. Returns the run
/// length (2–3 digits) when `chars[i]` starts such a run, else `None`.
/// Runs of 4+ digits never fragment: classic Japanese typesetting
/// rotates long numbers in full.
pub fn tatechuyoko_run(chars: &[char], i: usize) -> Option<usize> {
    let first = *chars.get(i)?;
    if !first.is_ascii_digit() {
        return None;
    }
    // Walk back to the run start so a 4+ digit run never fragments into
    // "1" + "234" (both callers scan left→right and consume units).
    let mut start = i;
    while start > 0 && chars[start - 1].is_ascii_digit() {
        start -= 1;
    }
    if start != i {
        return None;
    }
    let mut end = i;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    (2..=3).contains(&(end - start)).then_some(end - start)
}

/// Per-glyph scale inside a 縦中横 unit (`n` chars share one em cell).
pub fn tatechuyoko_scale(n: usize) -> f64 {
    1.0 / n as f64
}

/// True for the fullwidth brackets that Japanese faces set **rotated 90°**
/// in vertical writing via the `vert`/`vrt2` OpenType features
/// （）〔〕［］｛〈〉《》【】「」『』 … . The PDF/outline path gets the real
/// substituted glyph (see `vertical_open_type_outline`); the canvas and SVG
/// renderers — which draw live text, not outlines — use this set to rotate
/// the original glyph instead, which is visually the same result.
pub fn is_vert_rotated_char(ch: char) -> bool {
    matches!(
        ch,
        '（' | '）'
            | '〔'
            | '〕'
            | '［'
            | '］'
            | '｛'
            | '｝'
            | '〈'
            | '〉'
            | '《'
            | '》'
            | '【'
            | '】'
            | '〖'
            | '〗'
            | '〘'
            | '〙'
            | '〚'
            | '〛'
            | '「'
            | '」'
            | '『'
            | '』'
            | '｢'
            | '｣'
    )
}

/// True for the fullwidth punctuation the `halt` (Alternate Half Widths)
/// OpenType feature respaces to half an em: 、。，．・：；！？…‥ and the
/// halfwidth-adjacent forms. Brackets are deliberately absent — Japanese
/// faces keep （） at full width under `halt` (measured on Noto Sans JP),
/// so including them would desync the estimate model from the shaping.
pub fn is_halt_char(ch: char) -> bool {
    matches!(
        ch,
        '、' | '。'
            | '，'
            | '．'
            | '・'
            | '：'
            | '；'
            | '！'
            | '？'
            | '･'
            | '…'
            | '‥'
            | '—'
            | '―'
            | '‐'
            | '－'
            | '〜'
            | '～'
    )
}

/// One ruby (ルビ) group: `reading` set at half size beside the `len`
/// base characters starting at char index `start` of the base text.
#[derive(Debug, Clone, PartialEq)]
pub struct RubyAnnotation {
    /// Char index of the group's first base char.
    pub start: usize,
    /// How many base chars the reading covers.
    pub len: usize,
    /// The reading itself (kana / letters).
    pub reading: String,
}

/// Stripped ruby markup: base text, its annotations, and a per-char
/// "is markup" mask over the ORIGINAL text. The mask lets the vertical
/// wrapper keep markup inside its column strings (so every renderer
/// re-parses the same notation) while giving it no horizontal advance.
#[derive(Debug, Clone, PartialEq)]
pub struct RubyParse {
    pub base: String,
    pub anns: Vec<RubyAnnotation>,
    /// `true` = markup char (`｜《》` or a `(reading)` run).
    pub markup: Vec<bool>,
}

/// Strip ruby notation from `text`.
///
/// Two notations are understood:
/// - `｜漢字《かんじ》` — the reading spans everything between `｜`
///   and `《` (canonical, used for multi-char bases)
/// - `字(よみ)` — the reading attaches to the single preceding char
///
/// Unbalanced markup is kept verbatim: a stray `(` or `｜` is ordinary
/// text, and horizontal renders show it that way (only the vertical
/// pipeline renders readings).
pub fn parse_ruby_markup(text: &str) -> RubyParse {
    let chars: Vec<char> = text.chars().collect();
    let mut markup = vec![false; chars.len()];
    let mut base = String::new();
    let mut anns: Vec<RubyAnnotation> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        // ｜base《reading》
        if chars[i] == '\u{FF5C}' {
            let open = (i + 1..chars.len()).find(|&j| chars[j] == '\u{300A}');
            if let Some(open) = open {
                let close = (open + 1..chars.len()).find(|&j| chars[j] == '\u{300B}');
                if let Some(close) = close {
                    let reading: String = chars[open + 1..close].iter().collect();
                    let start = base.chars().count();
                    let len = open - (i + 1);
                    if len > 0 && !reading.is_empty() {
                        for b in &chars[i + 1..open] {
                            base.push(*b);
                        }
                        anns.push(RubyAnnotation {
                            start,
                            len,
                            reading,
                        });
                        markup[i] = true;
                        for m in markup.iter_mut().take(close + 1).skip(open) {
                            *m = true;
                        }
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
        // base(reading)
        if chars[i] == '(' {
            let close = (i + 1..chars.len()).find(|&j| chars[j] == ')');
            if let Some(close) = close {
                let reading: String = chars[i + 1..close].iter().collect();
                let start = base.chars().count();
                if !reading.is_empty() && start > 0 {
                    anns.push(RubyAnnotation {
                        start: start - 1,
                        len: 1,
                        reading,
                    });
                    for m in markup.iter_mut().take(close + 1).skip(i) {
                        *m = true;
                    }
                    i = close + 1;
                    continue;
                }
            }
        }
        base.push(chars[i]);
        i += 1;
    }
    RubyParse { base, anns, markup }
}

/// Convenience wrapper: base text plus its annotations.
pub fn parse_ruby(text: &str) -> (String, Vec<RubyAnnotation>) {
    let parsed = parse_ruby_markup(text);
    (parsed.base, parsed.anns)
}

/// Scale readings are typeset at (half size), matching 縦中横's pair
/// scaling and the canvas/galley path.
pub const RUBY_SCALE: f64 = 0.5;

/// Horizontal ruby: vertical centre of the reading, in em above the
/// base baseline (base ink tops out around 0.7em for CJK).
pub const RUBY_ABOVE_EM: f64 = 0.95;

/// Vertical ruby: horizontal centre of the reading strip, in em from
/// the base column's left edge (the strip spans 1em..1.5em).
pub const RUBY_STRIP_CENTER_EM: f64 = 1.25;

/// Laid-out text: drawable lines, how many fit, and the first baseline
/// origin in local coordinates.
pub struct TextLayout {
    pub lines: Vec<String>,
    /// Lines that fit (prefix of `lines`); the rest overflows.
    pub visible: usize,
    /// First-baseline origin (start-anchor x, baseline y) in local coords.
    pub origin: (f64, f64),
    /// Column index per line (all zero when uncolumned).
    pub col_of_line: Vec<usize>,
    /// Absolute x of each column's left edge (local coords).
    pub col_x: Vec<f64>,
    /// Width of each column.
    pub col_w: Vec<f64>,
    /// Hanging indent per line (DTP lists), in local units.
    pub line_indent: Vec<f64>,
    /// Runaround x shift per line (DTP text wrap), in local units.
    pub line_xoff: Vec<f64>,
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
    let normalized = normalize_text(text);
    let text = normalized.as_ref();
    let area = area.map(|a| a.sanitized());
    if style.vertical {
        return layout_text_vertical(text, style, area);
    }
    match area {
        None => {
            let runs = if style.word_wrap {
                if let Some(max_w) = style.max_width {
                    compute_wrapped_runs(text, style, max_w)
                } else {
                    text.split('\n').map(|s| (s.to_string(), 0.0)).collect()
                }
            } else {
                text.split('\n').map(|s| (s.to_string(), 0.0)).collect()
            };
            let visible = runs.len();
            let mut lines = Vec::with_capacity(visible);
            let mut line_indent = Vec::with_capacity(visible);
            let mut line_xoff = Vec::with_capacity(visible);
            for (line, indent) in runs {
                lines.push(line);
                line_indent.push(indent);
                line_xoff.push(0.0);
            }
            TextLayout {
                lines,
                visible,
                origin: (0.0, 0.0),
                col_of_line: vec![0; visible],
                col_x: vec![0.0],
                col_w: vec![f64::MAX],
                line_indent,
                line_xoff,
            }
        }
        Some(a) => {
            let widths = a.column_widths();
            let origins = a.column_origins();
            let ncols = widths.len().max(1);
            let runs = compute_wrapped_runs(text, style, widths[0]);
            let mut lines = Vec::with_capacity(runs.len());
            let mut line_indent = Vec::with_capacity(runs.len());
            let mut line_xoff = Vec::with_capacity(runs.len());
            for (line, indent) in runs {
                lines.push(line);
                line_indent.push(indent);
                line_xoff.push(0.0);
            }
            let line_h = style.effective_line_height().max(1e-6);
            let font_size = style.effective_font_size();
            // First baseline at the em-box top + font_size; a line fits
            // while its baseline stays inside the box.
            let per_col =
                ((((a.height - font_size) / line_h).floor() as isize) + 1).max(0) as usize;
            let visible = lines.len().min(per_col.saturating_mul(ncols));
            let mut col_of_line = Vec::with_capacity(lines.len());
            for i in 0..lines.len() {
                col_of_line.push((i / per_col.max(1)).min(ncols - 1));
            }
            TextLayout {
                lines,
                visible,
                origin: (origins[0], a.y + font_size),
                col_of_line,
                col_x: origins,
                col_w: widths,
                line_indent,
                line_xoff,
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
    let font_size = style.effective_font_size();
    let letter_spacing = style.effective_letter_spacing();
    // Advance of a char in a vertical column, plus extra advance at a
    // 和欧 boundary (~1/4em when `auto_spacing` is on).
    let char_adv = |prev: Option<char>, ch: char| {
        let mut adv = char_advance_styled(ch, style) * font_size + letter_spacing;
        if style.auto_spacing {
            if let Some(p) = prev {
                if is_ja_latin_boundary(p, ch) {
                    adv += ja_latin_gap_em(font_size, style.auto_spacing_em as f64);
                }
            }
        }
        adv
    };

    // Split into source lines, optionally wrapping each to the box height.
    // Wrapping honours Japanese 禁則 (see `wrap_vertical_run`).
    let mut columns: Vec<String> = Vec::new();
    match area {
        None => {
            for para in text.split('\n') {
                if style.word_wrap {
                    if let Some(max_h) = style.max_width {
                        // Wrap to `max_h`, kinsoku-aware.
                        let hang = hang_px(style);
                        columns.extend(wrap_vertical_run(
                            para,
                            char_adv,
                            font_size + letter_spacing,
                            max_h,
                            &hang,
                        ));
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
            let max_h = (a.height - font_size).max(font_size).max(1.0);
            for para in text.split('\n') {
                let hang = hang_px(style);
                columns.extend(wrap_vertical_run(
                    para,
                    char_adv,
                    font_size + letter_spacing,
                    max_h,
                    &hang,
                ));
            }
        }
    }

    // Capacity: how many columns fit in the box width (right edge starts at a.x + a.width).
    let (visible, origin) = match area {
        None => (columns.len(), (0.0, 0.0)),
        Some(a) => {
            let capacity =
                ((((a.width - font_size) / col_advance).floor() as isize) + 1).max(0) as usize;
            // First column's baseline sits near the right edge of the box (vertical-rl).
            let origin_x = a.x + a.width - font_size;
            (columns.len().min(capacity), (origin_x, a.y + font_size))
        }
    };

    let n = columns.len();
    // Per-column placement: column `i` advances left by `i * col_advance`
    // from the rightmost (first) column — so renderers can position
    // each column without re-deriving the layout.
    let col_of_line: Vec<usize> = (0..n).collect();
    let col_x: Vec<f64> = (0..n).map(|i| origin.0 - i as f64 * col_advance).collect();
    let col_w: Vec<f64> = vec![col_advance; n];
    TextLayout {
        lines: columns,
        visible,
        origin,
        col_of_line,
        col_x,
        col_w,
        line_indent: vec![0.0; n],
        line_xoff: vec![0.0; n],
    }
}

/// Greedy column wrap for vertical text, honouring Japanese 禁則:
/// a 行頭禁則 char that would start a column stays at the end of the
/// column above (追い込み), and 行末禁則 chars that would end a column
/// are pushed down to start the next one (追い出し). Columns may
/// overflow slightly when 追い込み applies — same as the horizontal
/// wrap. `char_adv(prev, ch)` must include letter-spacing and any
/// 和欧 boundary gap.
///
/// Ruby markup (`｜漢字《かんじ》` / `字(よみ)`) stays in the column
/// string — the vertical renderers re-parse it — but takes no advance,
/// so wrapping sees only the base characters.
fn wrap_vertical_run(
    para: &str,
    char_adv: impl Fn(Option<char>, char) -> f64,
    em_cell: f64,
    max_h: f64,
    hang: impl Fn(char) -> f64,
) -> Vec<String> {
    let parsed = parse_ruby_markup(para);
    let markup = parsed.markup;
    let chars: Vec<char> = para.chars().collect();
    let mut columns: Vec<String> = Vec::new();
    let mut col = String::new();
    let mut w = 0.0;
    let mut prev: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        // Ruby markup: keep it in the column, advance nothing.
        if markup[i] {
            col.push(chars[i]);
            i += 1;
            continue;
        }
        // 縦中横: 2–3 digits share one em cell.
        if let Some(n) = tatechuyoko_run(&chars, i) {
            let last = chars[i + n - 1];
            if !col.is_empty() && w + em_cell > max_h {
                columns.push(std::mem::take(&mut col));
                w = 0.0;
            }
            for u in &chars[i..i + n] {
                col.push(*u);
            }
            w += em_cell;
            prev = Some(last);
            i += n;
            continue;
        }
        let ch = chars[i];
        let cw = char_adv(prev, ch);
        // ぶら下げ: a trailing 閉じ約物 may overrun the column budget by
        // its ink (see `hang_px`), so the punctuation's ink hangs past the
        // column bottom instead of the whole em box.
        let next_hang = chars.get(i + 1).copied().unwrap_or('\0');
        let budget = max_h + hang(next_hang) + hang(ch);
        if !col.is_empty() && w + cw > budget && !kinsoku_cannot_start_line(ch) {
            // 追い出し: trailing opening bracket(s) start the next column.
            let mut head = String::new();
            while col.chars().count() > 1 && col.chars().last().is_some_and(kinsoku_cannot_end_line)
            {
                head.insert(0, col.pop().unwrap_or(ch));
            }
            columns.push(std::mem::take(&mut col));
            w = 0.0;
            prev = None;
            for m in head.chars() {
                col.push(m);
                w += char_adv(prev, m);
                prev = Some(m);
            }
        }
        col.push(ch);
        w += cw;
        if !is_zero_width(ch) {
            prev = Some(ch);
        }
        i += 1;
    }
    columns.push(col);
    columns
}

/// Distribute `text` across linked area-text frames (`frames` in link order).
/// Each frame receives as many laid-out lines/columns as it can hold; the
/// remainder overflows into the next frame. Returns one `TextLayout` per frame.
///
/// This is the core of threaded text stories (テキストスレッド). Callers resolve
/// the `next_frame` chain and pass areas in order.
pub fn layout_text_thread(text: &str, style: &TextStyle, frames: &[TextArea]) -> Vec<TextLayout> {
    if frames.is_empty() {
        return vec![layout_text(text, style, None)];
    }
    // Lay out once at the NARROWEST frame width, then slice lines per
    // frame. Narrow-wrapped lines always fit wider frames (ragged but
    // never overflowing); per-frame rewrapping would need char-level
    // resume across widths. Same-width stories (the common case) are
    // byte-identical to wrapping at their own width.
    let frames: Vec<TextArea> = frames.iter().map(|f| f.sanitized()).collect();
    let normalized = normalize_text(text);
    let text = normalized.as_ref();
    let font_size = style.effective_font_size();
    let min_width = frames
        .iter()
        .map(|f| f.width)
        .fold(f64::MAX, f64::min)
        .max(1.0);
    let mut first = frames[0];
    first.width = min_width;
    let full = layout_text(text, style, Some(first));
    let mut out = Vec::with_capacity(frames.len());
    let mut consumed = 0usize;
    for (i, frame) in frames.iter().enumerate() {
        let col_or_line = style.effective_line_height().max(1e-6);
        let per_col = if style.vertical {
            ((((frame.width - font_size) / col_or_line).floor() as isize) + 1).max(0) as usize
        } else {
            ((((frame.height - font_size) / col_or_line).floor() as isize) + 1).max(0) as usize
        };
        let ncols = if style.vertical {
            1
        } else {
            frame.cols.max(1) as usize
        };
        let per_frame = if i + 1 == frames.len() {
            // Last frame keeps whatever is left (may overflow for display).
            full.lines.len().saturating_sub(consumed)
        } else {
            per_col
                .saturating_mul(ncols)
                .min(full.lines.len().saturating_sub(consumed))
        };
        let end = (consumed + per_frame).min(full.lines.len());
        let slice: Vec<String> = full.lines[consumed..end].to_vec();
        let slice_indent: Vec<f64> = full.line_indent.get(consumed..end).unwrap_or(&[]).to_vec();
        let origin = if style.vertical {
            (frame.x + frame.width - font_size, frame.y + font_size)
        } else {
            (frame.x, frame.y + font_size)
        };
        // Column assignment inside this frame's slice. Vertical: one
        // column per laid-out line, advancing right→left.
        let (col_of_line, col_x, col_w) = if style.vertical {
            let advance = style.effective_line_height().max(1e-6);
            (
                (0..slice.len()).collect(),
                (0..slice.len())
                    .map(|i| origin.0 - i as f64 * advance)
                    .collect(),
                vec![advance; slice.len()],
            )
        } else {
            let xs = frame.column_origins();
            let ws = frame.column_widths();
            let assign: Vec<usize> = (0..slice.len())
                .map(|k| (k / per_col.max(1)).min(ncols - 1))
                .collect();
            (assign, xs, ws)
        };
        let n_lines = slice.len();
        out.push(TextLayout {
            visible: n_lines,
            lines: slice,
            origin,
            col_of_line,
            col_x,
            col_w,
            line_indent: slice_indent,
            line_xoff: vec![0.0; n_lines],
        });
        consumed = end;
    }
    out
}

fn affine_inverse(m: &[f64; 6]) -> Option<[f64; 6]> {
    let det = m[0] * m[3] - m[1] * m[2];
    if det.abs() < 1e-12 {
        return None;
    }
    Some([
        m[3] / det,
        -m[1] / det,
        -m[2] / det,
        m[0] / det,
        (m[2] * m[5] - m[3] * m[4]) / det,
        (m[1] * m[4] - m[0] * m[5]) / det,
    ])
}

/// Full layout entry point: thread flow, then runaround, then plain layout.
/// Renderers should call this instead of `layout_text`/`thread_frame_layout`
/// directly so every consumer agrees.
pub fn layout_text_full(
    doc: &super::Document,
    obj_id: &str,
    text: &str,
    style: &TextStyle,
    area: Option<TextArea>,
) -> TextLayout {
    if let Some(tl) = thread_frame_layout(doc, obj_id) {
        return tl;
    }
    if let Some(tl) = layout_text_wrapped(doc, obj_id, text, style, area) {
        return tl;
    }
    layout_text(text, style, area)
}

/// DTP runaround: single unthreaded single-column unlisted area frames
/// rewrap around `text_wrap` obstacles. Returns `None` when inapplicable
/// (threaded frames, columns, lists, vertical, point text, no obstacles),
/// so the common path is byte-identical to plain layout.
pub fn layout_text_wrapped(
    doc: &super::Document,
    obj_id: &str,
    text: &str,
    style: &TextStyle,
    area: Option<TextArea>,
) -> Option<TextLayout> {
    let a = area?.sanitized();
    if style.vertical || a.cols > 1 || style.list != ListStyle::None {
        return None;
    }
    // Resolve the frame anywhere in the tree, threading the world
    // transform: grouped area text wraps like top-level text.
    fn mat_mul(a: &[f64; 6], b: &[f64; 6]) -> [f64; 6] {
        [
            a[0] * b[0] + a[2] * b[1],
            a[1] * b[0] + a[3] * b[1],
            a[0] * b[2] + a[2] * b[3],
            a[1] * b[2] + a[3] * b[3],
            a[0] * b[4] + a[2] * b[5] + a[4],
            a[1] * b[4] + a[3] * b[5] + a[5],
        ]
    }
    fn find<'a>(
        objs: &'a [Object],
        obj_id: &str,
        parent: &[f64; 6],
    ) -> Option<(&'a Object, [f64; 6])> {
        for o in objs {
            let world = mat_mul(parent, &o.transform.matrix());
            if o.id == obj_id {
                return Some((o, world));
            }
            match &o.object_type {
                ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                    if let Some(hit) = find(children, obj_id, &world) {
                        return Some(hit);
                    }
                }
                _ => {}
            }
        }
        None
    }
    let ident = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut found = None;
    for layer in &doc.layers {
        if let Some(hit) = find(&layer.objects, obj_id, &ident) {
            found = Some(hit);
            break;
        }
    }
    let (_obj, to_doc) = found?;
    let to_local = affine_inverse(&to_doc)?;
    // Obstacles in frame-local space, gathered recursively so grouped
    // obstacles count too. `bounding_box` already includes the object's
    // OWN transform (never re-apply it); ancestors compose on top.
    fn gather_obstacles(
        objs: &[Object],
        parent: &[f64; 6],
        self_id: &str,
        to_local: &[f64; 6],
        out: &mut Vec<(f64, f64, f64, f64)>,
    ) {
        // Local affine helpers (mat_mul is defined in the enclosing scope).
        for other in objs {
            if !other.visible {
                continue;
            }
            let m = other.transform.matrix();
            let world = [
                parent[0] * m[0] + parent[2] * m[1],
                parent[1] * m[0] + parent[3] * m[1],
                parent[0] * m[2] + parent[2] * m[3],
                parent[1] * m[2] + parent[3] * m[3],
                parent[0] * m[4] + parent[2] * m[5] + parent[4],
                parent[1] * m[4] + parent[3] * m[5] + parent[5],
            ];
            match &other.object_type {
                ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                    gather_obstacles(children, &world, self_id, to_local, out);
                }
                _ => {}
            }
            if other.id == self_id || !other.text_wrap {
                continue;
            }
            let Some((mn, mx)) = other.bounding_box() else {
                continue;
            };
            // bbox is own-transform space: compose ancestors only.
            let corners = [(mn.x, mn.y), (mx.x, mn.y), (mx.x, mx.y), (mn.x, mx.y)];
            let wx: Vec<(f64, f64)> = corners
                .iter()
                .map(|&(x, y)| {
                    (
                        parent[0] * x + parent[2] * y + parent[4],
                        parent[1] * x + parent[3] * y + parent[5],
                    )
                })
                .collect();
            let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for &(x, y) in &wx {
                // Into frame-local space.
                let lx = to_local[0] * x + to_local[2] * y + to_local[4];
                let ly = to_local[1] * x + to_local[3] * y + to_local[5];
                x0 = x0.min(lx);
                y0 = y0.min(ly);
                x1 = x1.max(lx);
                y1 = y1.max(ly);
            }
            let mg = other.wrap_margin.max(0.0);
            out.push((x0 - mg, y0 - mg, x1 + mg, y1 + mg));
        }
    }
    let mut obstacles: Vec<(f64, f64, f64, f64)> = Vec::new();
    let ident = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        gather_obstacles(&layer.objects, &ident, obj_id, &to_local, &mut obstacles);
    }
    if obstacles.is_empty() {
        return None;
    }
    let line_h = style.effective_line_height().max(1e-6);
    let font_size = style.effective_font_size();
    let letter_spacing = style.effective_letter_spacing();
    let unit = |ch: char| char_advance_styled(ch, style) * font_size + letter_spacing;
    // Wrap paragraph by paragraph with live geometry.
    let mut lines: Vec<String> = Vec::new();
    let mut xoffs: Vec<f64> = Vec::new();
    let mut count = 0usize;
    let normalized = normalize_text(text);
    for paragraph in normalized.split('\n') {
        let chars: Vec<char> = paragraph.chars().collect();
        if chars.is_empty() {
            lines.push(String::new());
            xoffs.push(0.0);
            count += 1;
            continue;
        }
        // Geometry closure borrows obstacles + area; wrap one paragraph at
        // a time starting at the current visual index.
        let base = count;
        let a_ref = &a;
        let obs_ref = &obstacles;
        let lh = line_h;
        let mut g = |idx: usize, _first: bool| -> (f64, f64) {
            // Band containing the baseline: baselines sit at
            // a.y + font_size + idx*lh, glyphs extend ~line_h above.
            let y0 = a_ref.y + idx as f64 * lh + font_size - lh;
            let y1 = y0 + lh;
            let mut x0 = a_ref.x;
            let mut x1 = a_ref.x + a_ref.width;
            for e in obs_ref.iter() {
                // Skip obstacles fully above or below this line band.
                if e.3 <= y0 || e.1 >= y1 {
                    continue;
                }
                // Overlap in Y: shrink from the overlapped side. If the
                // obstacle covers the line start, push right; if it covers
                // the end, pull left; if it swallows the line, keep a
                // 1pt sliver so wrapping still terminates.
                if e.0 <= x0 + 1e-9 {
                    x0 = x0.max(e.2);
                } else if e.2 >= x1 - 1e-9 {
                    x1 = x1.min(e.0);
                } else {
                    // Middle obstacle: flow around the left part only
                    // (single-sided wrap; full contour wrap is out of scope).
                    x1 = x1.min(e.0);
                }
            }
            ((x0 - a_ref.x).max(0.0), (x1 - x0).max(1.0))
        };
        let hang = hang_px(style);
        for (s, x) in wrap_chars_g(
            &chars,
            &unit,
            if style.auto_spacing {
                ja_latin_gap_em(font_size, style.auto_spacing_em as f64)
            } else {
                0.0
            },
            hang,
            base,
            &mut g,
        ) {
            lines.push(s);
            xoffs.push(x);
            count += 1;
        }
    }
    let capacity = ((((a.height - font_size) / line_h).floor() as isize) + 1).max(0) as usize;
    let visible = lines.len().min(capacity);
    let n = lines.len();
    Some(TextLayout {
        lines,
        visible,
        origin: (a.x, a.y + font_size),
        col_of_line: vec![0; n],
        col_x: vec![a.x],
        col_w: vec![a.width],
        line_indent: vec![0.0; n],
        line_xoff: xoffs,
    })
}

/// One table-of-contents entry: heading text + 1-based page (artboard).
#[derive(Debug, Clone, PartialEq)]
pub struct TocEntry {
    pub text: String,
    pub page: Option<usize>,
    pub level: usize,
}

/// Collect headings for an auto table of contents: area/point text whose
/// font size meets `min_size` (overseas docs use size, not named styles,
/// since styles are copied on apply and carry no reference). Level comes
/// from relative size (largest = level 0). Text inside groups is included;
/// threaded tails are skipped (the head carries the story).
pub fn collect_toc_entries(doc: &super::Document, min_size: f64) -> Vec<TocEntry> {
    struct Raw {
        text: String,
        size: f64,
        page: Option<usize>,
    }
    let boards = doc.effective_artboards();
    let mut raw: Vec<Raw> = Vec::new();
    // Tails to skip: any id referenced as someone's next_frame.
    let mut tails: std::collections::HashSet<String> = std::collections::HashSet::new();
    fn scan_tails(obj: &Object, out: &mut std::collections::HashSet<String>) {
        match &obj.object_type {
            ObjectType::Text {
                next_frame: Some(n),
                ..
            } => {
                out.insert(n.clone());
            }
            ObjectType::Text { .. } => {}
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                for c in children {
                    scan_tails(c, out);
                }
            }
            _ => {}
        }
    }
    fn scan(
        obj: &Object,
        parent: &[f64; 6],
        boards: &[super::Artboard],
        min_size: f64,
        tails: &std::collections::HashSet<String>,
        raw: &mut Vec<Raw>,
    ) {
        let o = obj.transform.matrix();
        let w = [
            parent[0] * o[0] + parent[2] * o[1],
            parent[1] * o[0] + parent[3] * o[1],
            parent[0] * o[2] + parent[2] * o[3],
            parent[1] * o[2] + parent[3] * o[3],
            parent[0] * o[4] + parent[2] * o[5] + parent[4],
            parent[1] * o[4] + parent[3] * o[5] + parent[5],
        ];
        match &obj.object_type {
            ObjectType::Text {
                text, style, area, ..
            } => {
                if tails.contains(&obj.id) {
                    return;
                }
                if style.font_size < min_size {
                    return;
                }
                // Headings are single-line: first non-empty line.
                let head = text.lines().map(str::trim).find(|l| !l.is_empty());
                let Some(head) = head else {
                    return;
                };
                // Page = artboard containing the object's world center
                // (grouped text composes ancestors; local-only math put
                // it on the wrong page).
                let at = |x: f64, y: f64| (w[0] * x + w[2] * y + w[4], w[1] * x + w[3] * y + w[5]);
                let center = if let Some(a) = area {
                    at(a.x + a.width / 2.0, a.y + a.height / 2.0)
                } else {
                    at(0.0, 0.0)
                };
                let mut page = None;
                for (i, b) in boards.iter().enumerate() {
                    if center.0 >= b.x
                        && center.0 <= b.x + b.width
                        && center.1 >= b.y
                        && center.1 <= b.y + b.height
                    {
                        page = Some(i + 1);
                        break;
                    }
                }
                raw.push(Raw {
                    text: head.to_string(),
                    size: style.font_size,
                    page,
                });
            }
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                for c in children {
                    scan(c, &w, boards, min_size, tails, raw);
                }
            }
            _ => {}
        }
    }
    for layer in &doc.layers {
        for obj in &layer.objects {
            scan_tails(obj, &mut tails);
        }
    }
    let ident = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    for layer in &doc.layers {
        for obj in &layer.objects {
            scan(obj, &ident, &boards, min_size, &tails, &mut raw);
        }
    }
    if raw.is_empty() {
        return Vec::new();
    }
    // Level by size rank (largest = 0), stable document order otherwise.
    let mut sizes: Vec<f64> = raw.iter().map(|r| r.size).collect();
    sizes.sort_by(|a, b| b.total_cmp(a));
    sizes.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    raw.into_iter()
        .map(|r| TocEntry {
            text: r.text,
            page: r.page,
            level: sizes
                .iter()
                .position(|s| (*s - r.size).abs() < 0.5)
                .unwrap_or(0),
        })
        .collect()
}

/// Render TOC entries as text with dot leaders computed from estimated
/// advances (real dot-leader tabs need tab stops, which do not exist yet).
pub fn render_toc_text(entries: &[TocEntry], style: &TextStyle, width: f64) -> String {
    let font_size = style.effective_font_size();
    let letter_spacing = style.effective_letter_spacing();
    let unit = |ch: char| char_advance_styled(ch, style) * font_size + letter_spacing;
    let dot_w: f64 = ".".chars().map(&unit).sum::<f64>() + unit(' ');
    let mut out = Vec::new();
    for e in entries {
        let indent = "  ".repeat(e.level.min(3));
        let pageno = e
            .page
            .map(|p| format!("p{p}"))
            .unwrap_or_else(|| "–".to_string());
        let head_w: f64 = format!("{indent}{}", e.text).chars().map(&unit).sum();
        let tail_w: f64 = pageno.chars().map(&unit).sum();
        let dots = ((width - head_w - tail_w) / dot_w.max(1.0))
            .floor()
            .max(2.0) as usize;
        let leader: String = ". ".repeat(dots);
        out.push(format!("{indent}{} {leader}{pageno}", e.text));
    }
    out.join("\n")
}

/// Resolve the threaded story containing `obj_id` and return THIS frame's
/// layout (its slice of the head's text, positioned at its own area).
/// Returns `None` when the object is not part of a thread — callers fall
/// back to their own `layout_text`. Cycles degrade to single-frame layout.
///
/// The story text/style always come from the head frame; linked frames
/// contribute only their boxes. This is what makes overflow flow instead
/// of duplicating or vanishing.
pub fn thread_frame_layout(doc: &super::Document, obj_id: &str) -> Option<TextLayout> {
    // Collect (id, area, is_head_candidate) for area-text frames.
    struct Frame {
        id: String,
        text: String,
        style: TextStyle,
        area: TextArea,
        next: Option<String>,
    }
    let mut frames: Vec<Frame> = Vec::new();
    fn gather(obj: &Object, out: &mut Vec<Frame>) {
        match &obj.object_type {
            ObjectType::Text {
                text,
                style,
                area: Some(a),
                next_frame,
                ..
            } => {
                out.push(Frame {
                    id: obj.id.clone(),
                    text: text.clone(),
                    style: style.clone(),
                    area: *a,
                    next: next_frame.clone(),
                });
            }
            ObjectType::Text { .. } => {}
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                for c in children {
                    gather(c, out);
                }
            }
            _ => {}
        }
    }
    for layer in &doc.layers {
        for obj in &layer.objects {
            gather(obj, &mut frames);
        }
    }
    if !frames.iter().any(|f| f.id == obj_id) {
        return None;
    }
    // Referenced ids = non-heads. If obj is unreferenced it may still head
    // a chain (or stand alone without links).
    let referenced: std::collections::HashSet<&str> =
        frames.iter().filter_map(|f| f.next.as_deref()).collect();
    // Find the head: walk `next` links from obj with cycle guard; the head
    // is the unreferenced frame that reaches obj, or obj itself.
    let by_id: std::collections::HashMap<&str, &Frame> =
        frames.iter().map(|f| (f.id.as_str(), f)).collect();
    // Walk backwards: find who points at obj, iteratively, to the head.
    let mut head_id = obj_id;
    let mut seen = std::collections::HashSet::new();
    seen.insert(obj_id);
    loop {
        let mut pred = None;
        for f in &frames {
            if f.next.as_deref() == Some(head_id) && !seen.contains(f.id.as_str()) {
                pred = Some(f.id.as_str());
                break;
            }
        }
        match pred {
            Some(p) => {
                seen.insert(p);
                head_id = p;
            }
            None => break,
        }
    }
    let head = by_id.get(head_id)?;
    if head.next.is_none() && !referenced.contains(head_id) {
        // Standalone frame, no links either way: not a thread.
        return None;
    }
    // Forward chain from head (cycle-guarded).
    let mut chain: Vec<&Frame> = Vec::new();
    let mut cur: Option<&Frame> = Some(head);
    let mut seen2 = std::collections::HashSet::new();
    while let Some(f) = cur {
        if !seen2.insert(f.id.as_str()) {
            break;
        }
        chain.push(f);
        cur = f.next.as_deref().and_then(|id| by_id.get(id).copied());
    }
    let pos = chain.iter().position(|f| f.id == obj_id)?;
    let areas: Vec<TextArea> = chain.iter().map(|f| f.area).collect();
    let layouts = layout_text_thread(&head.text, &head.style, &areas);
    layouts.into_iter().nth(pos)
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
                out.elements
                    .push(PathElement::CurveTo(crate::core::path::BezierSegment {
                        start: s,
                        control1: c1,
                        control2: c2,
                        end: e,
                    }));
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
    compute_wrapped_runs(text, style, max_width)
        .into_iter()
        .map(|(line, _)| line)
        .collect()
}

/// Wrapped lines plus hanging indent per line (DTP lists): each paragraph
/// contributes its marker-prefixed first line and indent-matched
/// continuations. Numbered markers restart at 1 per call (per text object).
pub fn compute_wrapped_runs(text: &str, style: &TextStyle, max_width: f64) -> Vec<(String, f64)> {
    let font_size = style.effective_font_size();
    let letter_spacing = style.effective_letter_spacing();
    let unit = |ch: char| char_advance_styled(ch, style) * font_size + letter_spacing;
    let mut result: Vec<(String, f64)> = Vec::new();
    let mut number = 1u32;
    let normalized = normalize_text(text);
    for paragraph in normalized.split('\n') {
        let (prefix, indent_w) = match style.list {
            ListStyle::None => (String::new(), 0.0),
            ListStyle::Bullet => {
                let marker = "\u{2022} ".to_string();
                let w: f64 = marker.chars().map(&unit).sum();
                (marker, w)
            }
            ListStyle::Numbered => {
                let marker = format!("{number}. ");
                number += 1;
                let w: f64 = marker.chars().map(&unit).sum();
                (marker, w)
            }
        };
        let chars: Vec<char> = paragraph.chars().collect();
        if chars.is_empty() {
            // Empty paragraph keeps its marker slot (numbered counts it).
            if style.list == ListStyle::None {
                result.push((String::new(), 0.0));
            } else {
                result.push((prefix, 0.0));
            }
            continue;
        }
        let hang = hang_px(style);
        let raws = wrap_chars(
            &chars,
            &unit,
            if style.auto_spacing {
                ja_latin_gap_em(font_size, style.auto_spacing_em as f64)
            } else {
                0.0
            },
            &hang,
            max_width - prefix_visual_width(&prefix, &unit),
            max_width - indent_w,
        );
        for (i, raw) in raws.iter().enumerate() {
            if i == 0 {
                let mut line = prefix.clone();
                line.push_str(raw);
                result.push((line, 0.0));
            } else {
                result.push((raw.clone(), indent_w));
            }
        }
    }
    result
}

fn prefix_visual_width(prefix: &str, unit: &impl Fn(char) -> f64) -> f64 {
    prefix.chars().map(unit).sum()
}

/// Greedy wrap of one paragraph's chars: the first visual line may use
/// `first_max` width, continuation lines `rest_max` (hanging indent).
fn wrap_chars(
    chars: &[char],
    unit: &impl Fn(char) -> f64,
    gap: f64,
    hang: impl Fn(char) -> f64,
    first_max: f64,
    rest_max: f64,
) -> Vec<String> {
    wrap_chars_g(chars, unit, gap, hang, 0, &mut |_, first| {
        (0.0, if first { first_max } else { rest_max })
    })
    .into_iter()
    .map(|(s, _)| s)
    .collect()
}

/// ぶら下げ budget helper: pixels a hanging char may overrun (0 when the
/// style disables it, or when the char is not a 閉じ約物).
fn hang_px(style: &TextStyle) -> impl Fn(char) -> f64 + '_ {
    let font_size = style.effective_font_size();
    move |ch: char| {
        if style.burasage {
            burasage_hang(ch) * font_size
        } else {
            0.0
        }
    }
}

/// Greedy wrap with per-visual-line geometry: `geom(line_idx, is_first)`
/// returns the line's x offset and available width (DTP runaround narrows
/// and shifts lines around obstacles). Returns `(line, xoff)` pairs.
fn wrap_chars_g(
    chars: &[char],
    unit: &impl Fn(char) -> f64,
    gap: f64,
    hang: impl Fn(char) -> f64,
    base_idx: usize,
    geom: &mut dyn FnMut(usize, bool) -> (f64, f64),
) -> Vec<(String, f64)> {
    let mut result = Vec::new();
    let mut line_start = 0usize;
    let mut i = 0usize;
    let mut first = true;
    // Visual-line counter (paragraph-continuation aware via `base_idx`).
    let mut vidx = base_idx;
    // Byte/char width of the current line for quick slicing.
    let mut line_w = 0.0f64;
    // Last index (exclusive end of line) where a break is allowed.
    let mut last_break: Option<usize> = None;
    while i < chars.len() {
        let ch = chars[i];
        let mut w = unit(ch);
        // 和欧間: visual gap between a Japanese and a Latin glyph on the
        // same (visual) line.
        if gap > 0.0 && i > line_start {
            let prev = chars[i - 1];
            if is_ja_latin_boundary(prev, ch) {
                w += gap;
            }
        }
        let (_, max_width) = geom(vidx, first);
        // ぶら下げ: when the NEXT char is a hanging 閉じ約物, this char must
        // still fit the nominal width — the punctuation's ink then overruns
        // into the margin by `hang` instead of the whole em box.
        let next_hang = chars.get(i + 1).copied().unwrap_or('\0');
        let budget = max_width + hang(next_hang) + hang(ch);
        // Would this char overflow the line?
        if line_w + w > budget && i > line_start {
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
            let (ex, _) = geom(vidx, first);
            result.push((chars[line_start..end].iter().collect(), ex));
            // Skip a single leading space/tab on the new line (Western
            // word-wrap convention); CJK needs no such trimming.
            line_start = end;
            if line_start < chars.len() && (chars[line_start] == ' ' || chars[line_start] == '\t') {
                line_start += 1;
            }
            i = line_start;
            line_w = 0.0;
            last_break = None;
            first = false;
            vidx += 1;
            continue;
        }
        line_w += w;
        // A break is allowed *after* this char when the next char may
        // legally start a line and this char may legally end one. Tabs break
        // like spaces (TSV pastes otherwise never wrap).
        let next_ok = i + 1 >= chars.len() || !kinsoku_cannot_start_line(chars[i + 1]);
        if (ch == ' ' || ch == '\t' || is_fullwidth(ch)) && !kinsoku_cannot_end_line(ch) && next_ok
        {
            last_break = Some(i + 1);
        }
        i += 1;
    }
    let (ex, _) = geom(vidx, first);
    result.push((chars[line_start..].iter().collect(), ex));
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
    /// DTP runaround: text in other frames flows around this object's box.
    #[serde(default)]
    pub text_wrap: bool,
    /// Runaround margin in document units.
    #[serde(default = "default_wrap_margin")]
    pub wrap_margin: f64,
}

fn default_wrap_margin() -> f64 {
    6.0
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
    ) -> Self {
        Self {
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
            text_wrap: false,
            wrap_margin: 6.0,
            locked: false,
        }
    }

    pub fn new_pixel_art(name: &str, x: f64, y: f64, pixels: crate::core::pixel::PixelArt) -> Self {
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
            text_wrap: false,
            wrap_margin: 6.0,
            locked: false,
        }
    }

    pub fn new_mesh(
        name: &str,
        x: f64,
        y: f64,
        mesh: crate::core::gradient_mesh::MeshGradient,
    ) -> Self {
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
            text_wrap: false,
            wrap_margin: 6.0,
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
            text_wrap: false,
            wrap_margin: 6.0,
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
                text,
                font_size,
                style,
                area,
                ..
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
            ObjectType::Envelope {
                source,
                kind,
                amount,
            } => deform_path_data(&source.to_path_data(), *kind, amount.clamp(-1.0, 1.0)),
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
                text,
                font_size,
                style,
                area,
                ..
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
            }
            ObjectType::Envelope { .. } => {
                let poly = self.to_path_data().to_polygon(8);
                crate::core::geometry::point_in_polygon(lx, ly, &poly)
            }
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

    /// [`bounding_box`] grown by half the stroke width — Illustrator's
    /// *プレビュー境界*.
    ///
    /// The pad is per axis: the stroke is laid out in object space, so a
    /// horizontal edge's thickness travels with `scale_x` and a vertical
    /// one's with `scale_y`. [`Self::visual_scale`] (the geometric mean)
    /// would be right only for a uniform resize.
    ///
    /// Shadow and glow are deliberately left out: they have no finite extent
    /// (a glow's spread depends on the blur) and including them would make
    /// the selection box jump every time an effect is retuned.
    pub fn preview_bounds(&self) -> Option<(AnchorPoint, AnchorPoint)> {
        let (min, max) = self.bounding_box()?;
        let (pad_x, pad_y) = match &self.stroke {
            Some(s) if s.width > 0.0 => (
                s.width * self.transform.scale_x.abs() * 0.5,
                s.width * self.transform.scale_y.abs() * 0.5,
            ),
            _ => (0.0, 0.0),
        };
        if pad_x <= 0.0 && pad_y <= 0.0 {
            return Some((min, max));
        }
        Some((
            AnchorPoint::new(min.x - pad_x, min.y - pad_y),
            AnchorPoint::new(max.x + pad_x, max.y + pad_y),
        ))
    }

    /// The box this object is *measured* by: its [`preview_bounds`] when
    /// 「プレビュー境界を使用」is on, the bare geometry otherwise.
    ///
    /// Alignment, distribution, guides, snapping, measurements and the
    /// selection box all go through here, so "how big is this object" has
    /// exactly one answer per preference instead of a different one in every
    /// panel.
    pub fn measured_bounds(&self, use_preview_bounds: bool) -> Option<(AnchorPoint, AnchorPoint)> {
        if use_preview_bounds {
            self.preview_bounds()
        } else {
            self.bounding_box()
        }
    }

    /// The scale factor every *visual* attribute is drawn at: the geometric
    /// mean of the transform's axis scales (exactly the axis scale when the
    /// resize was uniform).
    pub fn visual_scale(&self) -> f64 {
        (self.transform.scale_x.abs() * self.transform.scale_y.abs()).sqrt()
    }

    /// Keep the absolute-valued attributes in place across a change of
    /// [`Self::visual_scale`] — the counterpart of a resize.
    ///
    /// `ratio` is *new ÷ old*. Each 環境設定 switch decides whether the
    /// attribute is meant to ride the object's scale: **on** → it is, so
    /// nothing is touched; **off** → the stored value is divided by `ratio`,
    /// so `attribute × visual_scale` — what the canvas and the exporter draw
    /// — stays exactly where the user last saw it.
    pub fn apply_scale_change(
        &mut self,
        ratio: f64,
        scale_corners: bool,
        scale_strokes_effects: bool,
    ) {
        if !ratio.is_finite() || ratio <= 0.0 || (ratio - 1.0).abs() < 1e-12 {
            return;
        }
        let k = 1.0 / ratio;
        if !scale_strokes_effects {
            if let Some(s) = self.stroke.as_mut() {
                s.width *= k;
            }
            if let Some(sh) = self.shadow.as_mut() {
                sh.offset_x *= k;
                sh.offset_y *= k;
                sh.blur_radius *= k;
            }
            if let Some(g) = self.glow.as_mut() {
                g.radius *= k;
            }
            self.appearance.counter_scale(k);
        }
        if !scale_corners {
            if let ObjectType::Rectangle { corner_radius, .. } = &mut self.object_type {
                *corner_radius *= k;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BlendMode, DropShadow, Object, ObjectType, StrokeStyle};
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

    /// A stroked, rounded rectangle with a shadow and a blur — the three
    /// things the transform switches decide about.
    fn visual_fixture() -> Object {
        use crate::core::effects::{BlurEffect, VectorEffect};
        let mut obj = Object::new_rect("R", 0.0, 0.0, 100.0, 50.0, 20.0);
        obj.stroke = Some(StrokeStyle {
            width: 4.0,
            ..Default::default()
        });
        obj.shadow = Some(DropShadow::default()); // offset 6/6, blur 8
        obj.appearance
            .push(VectorEffect::Blur(BlurEffect { radius: 6.0 }));
        obj
    }

    fn corner_radius_of(obj: &Object) -> f64 {
        match &obj.object_type {
            ObjectType::Rectangle { corner_radius, .. } => *corner_radius,
            other => panic!("expected a rectangle, got {other:?}"),
        }
    }

    /// 「角を拡大・縮小」・「線幅と効果を拡大・縮小」off: doubling the object
    /// halves what is stored, so *stored × scale* — what is drawn — stays.
    #[test]
    fn scale_change_holds_visuals_absolute() {
        let mut obj = visual_fixture();
        obj.apply_scale_change(2.0, false, false);
        assert_eq!(corner_radius_of(&obj), 10.0, "corner radius halves");
        assert_eq!(obj.stroke.as_ref().unwrap().width, 2.0, "stroke halves");
        let sh = obj.shadow.as_ref().unwrap();
        assert_eq!((sh.offset_x, sh.offset_y, sh.blur_radius), (3.0, 3.0, 4.0));
        assert_eq!(obj.appearance.has_blur(), Some(3.0), "blur radius halves");

        // …and the check that matters: the drawn size did not move.
        assert_eq!(corner_radius_of(&obj) * 2.0, 20.0);
        assert_eq!(obj.stroke.as_ref().unwrap().width * 2.0, 4.0);
    }

    /// Switches on: the transform alone carries the size, nothing is edited.
    #[test]
    fn scale_change_leaves_visuals_when_the_switches_are_on() {
        let mut obj = visual_fixture();
        obj.apply_scale_change(2.0, true, true);
        assert_eq!(corner_radius_of(&obj), 20.0);
        assert_eq!(obj.stroke.as_ref().unwrap().width, 4.0);
        assert_eq!(obj.appearance.has_blur(), Some(6.0));
    }

    /// The two switches are independent.
    #[test]
    fn scale_change_switches_are_independent() {
        let mut obj = visual_fixture();
        obj.apply_scale_change(2.0, true, false);
        assert_eq!(corner_radius_of(&obj), 20.0, "corners left alone");
        assert_eq!(obj.stroke.as_ref().unwrap().width, 2.0, "stroke halved");

        // A degenerate ratio (no scale change, or a broken one) is a no-op.
        let mut same = visual_fixture();
        same.apply_scale_change(1.0, false, false);
        assert_eq!(corner_radius_of(&same), 20.0);
        same.apply_scale_change(f64::NAN, false, false);
        assert_eq!(corner_radius_of(&same), 20.0);
    }

    /// Non-uniform resize: each axis carries its own part of the stroke.
    #[test]
    fn preview_bounds_pads_each_axis_with_its_own_scale() {
        let mut obj = visual_fixture(); // 100×50, stroke 4
        obj.transform.scale_x = 3.0;
        obj.transform.scale_y = 1.0;
        let (min, max) = obj.preview_bounds().unwrap();
        // x: geometry 0..300, pad 4·3/2 = 6 a side. y: 0..50, pad 4·1/2 = 2.
        assert_eq!((min.x, min.y, max.x, max.y), (-6.0, -2.0, 306.0, 52.0));
        // …whereas the geometric mean (4·√3/2 ≈ 3.46) would pad x too.
        assert_ne!(min.x, -4.0 * 3f64.sqrt() * 0.5);
    }

    /// The measurement every panel agrees on.
    #[test]
    fn measured_bounds_follows_the_preference() {
        let mut obj = visual_fixture();
        obj.transform.scale_x = 2.0;
        obj.transform.scale_y = 2.0;
        assert_eq!(
            obj.measured_bounds(false),
            obj.bounding_box(),
            "geometry-only"
        );
        assert_eq!(
            obj.measured_bounds(true),
            obj.preview_bounds(),
            "stroke included"
        );
        // With 「プレビュー境界を使用」on the box is the bigger one.
        let (_, geo) = obj.measured_bounds(false).unwrap();
        let (_, prev) = obj.measured_bounds(true).unwrap();
        assert!(prev.x > geo.x && prev.y > geo.y);
    }

    #[test]
    fn preview_bounds_add_half_a_stroke_that_scales() {
        let mut obj = visual_fixture();
        // Geometry alone: the rectangle, no pad.
        let (min, max) = obj.bounding_box().unwrap();
        assert_eq!((min.x, min.y, max.x, max.y), (0.0, 0.0, 100.0, 50.0));
        // Preview bounds: half the stroke (4 / 2 = 2) on every side.
        let (min, max) = obj.preview_bounds().unwrap();
        assert_eq!((min.x, min.y, max.x, max.y), (-2.0, -2.0, 102.0, 52.0));
        // Doubling the object doubles the pad too.
        obj.transform.scale_x = 2.0;
        obj.transform.scale_y = 2.0;
        let (min, max) = obj.preview_bounds().unwrap();
        assert_eq!((min.x, min.y, max.x, max.y), (-4.0, -4.0, 204.0, 104.0));
        // An unstroked object measures exactly like its geometry.
        obj.stroke = None;
        assert_eq!(obj.preview_bounds(), obj.bounding_box());
        assert_eq!(obj.visual_scale(), 2.0);
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

    #[test]
    fn columns_split_lines_across_origins() {
        use super::{layout_text, TextArea, TextStyle};
        let style = TextStyle {
            font_size: 12.0,
            ..Default::default()
        };
        let text: String = (0..10)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut area = TextArea::new(0.0, 0.0, 400.0, 60.0);
        area.cols = 2;
        area.gutter = 20.0;
        let one = layout_text(&text, &style, Some(TextArea::new(0.0, 0.0, 400.0, 60.0)));
        let two = layout_text(&text, &style, Some(area));
        // Two columns hold more lines than one.
        assert!(two.visible >= one.visible);
        // Column origins: second starts after first width + gutter.
        assert_eq!(two.col_x.len(), 2);
        assert!((two.col_x[1] - two.col_x[0] - two.col_w[0] - 20.0).abs() < 1e-6);
        // Lines past the first column capacity carry column index 1.
        assert!(two.col_of_line.contains(&1));
        assert_eq!(two.col_of_line.len(), two.lines.len());
    }

    #[test]
    fn list_markers_prefix_and_hang() {
        use super::{compute_wrapped_runs, ListStyle, TextStyle};
        let mut style = TextStyle {
            font_size: 12.0,
            ..Default::default()
        };
        style.list = ListStyle::Bullet;
        let runs = compute_wrapped_runs("a\nb", &style, 500.0);
        assert_eq!(runs.len(), 2);
        assert!(runs[0].0.starts_with("\u{2022} "));
        assert!(runs[1].0.starts_with("\u{2022} "));
        assert_eq!(runs[0].1, 0.0);
        style.list = ListStyle::Numbered;
        let runs = compute_wrapped_runs("a\nb", &style, 500.0);
        assert!(runs[0].0.starts_with("1. "));
        assert!(runs[1].0.starts_with("2. "));
        // Narrow width: continuation hangs by the marker width.
        let runs = compute_wrapped_runs("aaa bbb ccc ddd", &style, 40.0);
        assert!(runs.len() >= 2);
        assert!(runs[1].1 > 0.0, "hanging indent: {runs:?}");
    }

    #[test]
    fn runaround_narrows_lines_around_obstacles() {
        use super::{layout_text_full, TextArea, TextStyle};
        use crate::core::document::Document;
        let style = TextStyle {
            font_size: 12.0,
            ..Default::default()
        };
        let text = "aaa bbb ccc ddd eee fff ggg hhh iii jjj kkk lll";
        let area = TextArea::new(0.0, 0.0, 400.0, 400.0);
        // Obstacle covering the right half of the top ~3 lines.
        let mut rock = Object::new_rect("R", 250.0, 0.0, 150.0, 60.0, 0.0);
        rock.text_wrap = true;
        let mut frame = Object::new_text("F", text, 0.0, 0.0, 12.0);
        let frame_id = frame.id.clone();
        if let ObjectType::Text {
            style: s, area: a, ..
        } = &mut frame.object_type
        {
            *s = style.clone();
            *a = Some(area);
        }
        let mut doc = Document::default();
        doc.add_object(frame);
        doc.add_object(rock);
        // Without the flag there is no shift.

        let plain = layout_text_full(&doc, &frame_id, text, &style, Some(area));
        assert!(plain.line_xoff.iter().all(|&x| x == 0.0));
        // Full width: first line holds several words.
        let first_len = plain.lines[0].len();
        assert!(first_len > 10, "{}", plain.lines[0]);
        // The obstacle narrows the top lines: one full-width line
        // becomes two, with all characters preserved and no x-shift
        // (right-side obstacle only shrinks width).
        assert!(plain.lines.len() >= 2, "wraps around: {:?}", plain.lines);
        let chars: usize = plain.lines.iter().map(|l| l.len()).sum();
        assert!(chars >= 40, "no text lost: {chars}");
        assert!(
            plain.line_xoff.iter().all(|&x| x == 0.0),
            "right-side obstacle needs no shift"
        );
        // Left-side obstacle pushes lines right (positive x-shift).
        let mut left_rock = Object::new_rect("L", 0.0, 0.0, 150.0, 60.0, 0.0);
        left_rock.text_wrap = true;
        let mut doc2 = Document::default();
        // Rebuild frame (ids must be fresh for the new doc).
        let mut frame2 = Object::new_text("F2", text, 0.0, 0.0, 12.0);
        let fid2 = frame2.id.clone();
        if let ObjectType::Text {
            style: s, area: a, ..
        } = &mut frame2.object_type
        {
            *s = style.clone();
            *a = Some(area);
        }
        doc2.add_object(frame2);
        doc2.add_object(left_rock);
        let shifted = super::layout_text_full(&doc2, &fid2, text, &style, Some(area));
        assert!(
            shifted.line_xoff.iter().take(2).all(|&x| x > 100.0),
            "left obstacle shifts: {:?}",
            shifted.line_xoff
        );
    }

    #[test]
    fn toc_collects_headings_with_pages() {
        use super::{collect_toc_entries, render_toc_text, TextStyle};
        use crate::core::document::Document;
        let mut doc = Document {
            width: 400.0,
            height: 600.0,
            ..Default::default()
        };
        let big = TextStyle {
            font_size: 24.0,
            ..Default::default()
        };
        let small = TextStyle {
            font_size: 12.0,
            ..Default::default()
        };
        let mut h1 = Object::new_text_with_style("H1", "Title", 10.0, 10.0, big.clone());
        let _ = &mut h1;
        let mut b = Object::new_text_with_style("B", "body", 10.0, 100.0, small.clone());
        let _ = &mut b;
        let mut h2 = Object::new_text_with_style("H2", "Next", 10.0, 400.0, big.clone());
        let _ = &mut h2;
        doc.add_object(h1);
        doc.add_object(b);
        doc.add_object(h2);
        let entries = collect_toc_entries(&doc, 18.0);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "Title");
        assert_eq!(entries[1].text, "Next");
        assert_eq!(entries[0].level, entries[1].level);
        let body = render_toc_text(&entries, &small, 360.0);
        assert!(body.contains("Title") && body.contains("Next"));
        assert!(body.contains('p'), "page refs: {body}");
    }

    #[test]
    fn thread_flow_splits_overflow_across_frames() {
        use super::{layout_text, TextArea, TextStyle};
        use crate::core::document::Document;
        let style = TextStyle {
            font_size: 12.0,
            ..Default::default()
        };
        // Long text, small head frame: must flow into the tail.
        let text: String = (0..20)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let area1 = TextArea::new(0.0, 0.0, 200.0, 30.0);
        let area2 = TextArea::new(0.0, 100.0, 200.0, 300.0);
        let mut head = Object::new_text("H", &text, 0.0, 0.0, 12.0);
        let mut tail = Object::new_text("T", "", 0.0, 0.0, 12.0);
        let head_id = head.id.clone();
        let tail_id = tail.id.clone();
        if let ObjectType::Text {
            style: s,
            area,
            next_frame,
            ..
        } = &mut head.object_type
        {
            *s = style.clone();
            *area = Some(area1);
            *next_frame = Some(tail_id.clone());
        }
        if let ObjectType::Text { style: s, area, .. } = &mut tail.object_type {
            *s = style.clone();
            *area = Some(area2);
        }
        let mut doc = Document::default();
        doc.add_object(head);
        doc.add_object(tail);
        let h = super::thread_frame_layout(&doc, &head_id).expect("head resolves");
        let t2 = super::thread_frame_layout(&doc, &tail_id).expect("tail resolves");
        // Head holds a prefix, tail the rest; together they cover all lines.
        assert!(!h.lines.is_empty() && !t2.lines.is_empty());
        assert_eq!(h.lines.len() + t2.lines.len(), 20);
        // Tail is positioned at its own box, not the head's.
        assert!((t2.origin.0 - 0.0).abs() < 1e-9);
        assert!((t2.origin.1 - (100.0 + 12.0)).abs() < 1e-9);
        // Standalone frame (no links): no thread layout.
        let mut solo = Object::new_text("S", "hi", 0.0, 0.0, 12.0);
        if let ObjectType::Text { style: s, area, .. } = &mut solo.object_type {
            *s = style.clone();
            *area = Some(TextArea::new(0.0, 0.0, 200.0, 200.0));
        }
        let mut doc2 = Document::default();
        doc2.add_object(solo);
        let sid = doc2.all_objects().next().unwrap().1.id.clone();
        assert!(super::thread_frame_layout(&doc2, &sid).is_none());
        let _ = layout_text(&text, &style, Some(area1)).lines.len();
    }

    #[test]
    fn newline_normalization_collapses_crlf_cr_and_separators() {
        use super::normalize_text;
        assert_eq!(normalize_text("a\r\nb"), "a\nb");
        assert_eq!(normalize_text("a\rb"), "a\nb");
        assert_eq!(normalize_text("a b"), "a\nb");
        assert_eq!(normalize_text("a b"), "a\nb");
        assert_eq!(normalize_text("plain"), "plain");
    }

    #[test]
    fn crlf_wraps_and_measures_like_lf() {
        use super::{compute_wrapped_lines, text_block_size_with_style, TextStyle};
        let style = TextStyle {
            font_size: 10.0,
            word_wrap: true,
            max_width: Some(25.0),
            ..Default::default()
        };
        let lf = compute_wrapped_lines("あいう\r\nえお", &style, 25.0);
        let crlf = compute_wrapped_lines("あいう\nえお", &style, 25.0);
        assert_eq!(lf, crlf, "CRLF must not leave a stray \\r glyph");
        assert_eq!(crlf.concat(), "あいう\nえお".replace('\n', ""));
        let (w1, h1) = text_block_size_with_style("a\r\nb", &TextStyle::default());
        let (w2, h2) = text_block_size_with_style("a\nb", &TextStyle::default());
        assert_eq!((w1, h1), (w2, h2));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn hostile_style_values_stay_finite() {
        use super::TextStyle;
        let mut style = TextStyle::default();
        style.font_size = f64::NAN;
        style.letter_spacing = f64::INFINITY;
        style.line_height = Some(-2.0);

        assert!(style.effective_font_size().is_finite());
        assert!(style.effective_font_size() > 0.0);
        assert_eq!(style.effective_letter_spacing(), 0.0);
        assert!(style.effective_line_height().is_finite());
        assert!(style.effective_line_height() > 0.0);
        style.line_height = Some(f64::NAN);
        assert!((style.effective_line_height() - 1.2 * style.effective_font_size()).abs() < 1e-9);
        // Layout and measurement never produce NaN from hostile styles.
        let layout = super::layout_text("あいうえお", &style, None);
        assert!(!layout.lines.is_empty());
        let (w, h) = super::text_block_size_with_style("あいうえお", &style);
        assert!(w.is_finite() && h.is_finite(), "w={w} h={h}");
    }

    #[test]
    fn variation_settings_reject_non_finite() {
        use super::TextStyle;
        let parsed =
            TextStyle::parse_variation_settings_css("\"wght\" NaN, \"wdth\" inf, \"opsz\" 12");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].axis, "opsz");
        let mut style = TextStyle::default();
        style.set_variation("wght", f64::NAN);
        assert!(style.variation("wght").is_none(), "NaN must not be stored");
        style.set_variation("wght", 700.0);
        assert_eq!(style.variation("wght"), Some(700.0));
    }

    #[test]
    fn emoji_advance_full_em_and_format_chars_zero() {
        use super::char_advance_estimate;
        assert_eq!(char_advance_estimate('😀'), 1.0, "emoji sets fullwidth");
        assert_eq!(
            char_advance_estimate('■'),
            1.0,
            "geometric shapes fullwidth"
        );
        assert_eq!(
            char_advance_estimate('\u{FE0F}'),
            0.0,
            "VS16 rides the base glyph"
        );
        assert_eq!(
            char_advance_estimate('\u{200D}'),
            0.0,
            "ZWJ rides the base glyph"
        );
        assert_eq!(
            char_advance_estimate('\u{E0101}'),
            0.0,
            "IVS rides the base glyph"
        );
        assert_eq!(
            char_advance_estimate('\u{0301}'),
            0.0,
            "combining mark rides the base"
        );
        assert_eq!(
            char_advance_estimate('ｱ'),
            0.6,
            "halfwidth kana stays narrow"
        );
    }

    #[test]
    fn kinsoku_covers_halfwidth_and_quotes() {
        use super::{compute_wrapped_lines, TextStyle};
        let style = TextStyle {
            font_size: 10.0,
            word_wrap: true,
            max_width: Some(10.0),
            ..Default::default()
        };
        // 1-char width: every char on its own line unless kinsoku glues it.
        for text in ["あ｣あ", "あｰあ", "あ—あ", "あ「あ", "あ｢あ"] {
            let lines = compute_wrapped_lines(text, &style, 10.0);
            assert_eq!(lines.concat(), text, "no text lost: {lines:?}");
        }
        let lines = compute_wrapped_lines("あいう｣えお", &style, 25.0);
        for l in &lines {
            assert!(
                !l.starts_with('｣'),
                "halfwidth closing never starts a line: {lines:?}"
            );
        }
        let lines = compute_wrapped_lines("あいう｢えお", &style, 35.0);
        for l in &lines {
            assert!(
                !l.ends_with('｢'),
                "halfwidth opening never ends a line: {lines:?}"
            );
        }
    }

    #[test]
    fn tabs_wrap_like_spaces() {
        use super::{compute_wrapped_lines, TextStyle};
        let style = TextStyle {
            font_size: 10.0,
            word_wrap: true,
            max_width: Some(30.0),
            ..Default::default()
        };
        let lines = compute_wrapped_lines("aa\tbb\tcc", &style, 30.0);
        assert_eq!(lines.concat().replace('\t', ""), "aabbcc");
        assert!(lines.len() >= 2, "tabs must break: {lines:?}");
    }

    #[test]
    fn text_area_sanitized_kills_nan() {
        use super::TextArea;
        let hostile = TextArea {
            x: f64::NAN,
            y: f64::INFINITY,
            width: f64::NAN,
            height: -5.0,
            cols: 0,
            gutter: f64::NAN,
        };
        let clean = hostile.sanitized();
        assert!(clean.x.is_finite() && clean.y.is_finite());
        assert!(clean.width >= 1.0 && clean.height >= 1.0);
        assert!(clean.cols >= 1 && clean.gutter.is_finite());
        // Sanitized areas flow through layout without NaN origins.
        let layout = super::layout_text("あいう", &super::TextStyle::default(), Some(hostile));
        assert!(layout.origin.0.is_finite() && layout.origin.1.is_finite());
    }
}
