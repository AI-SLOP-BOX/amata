use rhai::Engine;

use crate::core::document::{BlendMode, Document, Layer, Object, ObjectType, Transform};
use crate::core::effects::{DropShadow, GlowEffect};
use crate::core::path::{
    AnchorPoint, BezierSegment, FillRule, FillStyle, FillType, GradientStop, LinearGradient,
    PathData, PathElement, StrokeStyle,
};
use crate::core::state::AppState;
use crate::io::svg::parse_svg_color;

pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Bound untrusted script execution: infinite loops / exponential
        // expansion would otherwise hang the UI thread, and unbounded
        // collections could exhaust memory.
        engine.set_max_operations(500_000);
        engine.set_max_call_levels(64);
        engine.set_max_string_size(1_000_000);
        engine.set_max_array_size(100_000);
        engine.set_max_map_size(10_000);

        // Math & geometry generation functions
        engine.register_fn(
            "rect",
            |x: f64, y: f64, w: f64, h: f64| -> (f64, f64, f64, f64) { (x, y, w, h) },
        );
        engine.register_fn(
            "circle",
            |cx: f64, cy: f64, r: f64| -> (f64, f64, f64, f64) { (cx, cy, r, r) },
        );
        engine.register_fn("lerp", |a: f64, b: f64, t: f64| -> f64 { a + (b - a) * t });
        engine.register_fn("deg_to_rad", |d: f64| -> f64 { d.to_radians() });
        engine.register_fn("rad_to_deg", |r: f64| -> f64 { r.to_degrees() });
        engine.register_fn("tau", || -> f64 { std::f64::consts::TAU });
        engine.register_fn("pi", || -> f64 { std::f64::consts::PI });
        engine.register_fn("sin", |x: f64| -> f64 { x.sin() });
        engine.register_fn("cos", |x: f64| -> f64 { x.cos() });
        engine.register_fn("sqrt", |x: f64| -> f64 { x.sqrt() });
        engine.register_fn("abs", |x: f64| -> f64 { x.abs() });
        engine.register_fn("min", |a: f64, b: f64| -> f64 { a.min(b) });
        engine.register_fn("max", |a: f64, b: f64| -> f64 { a.max(b) });

        // Procedural design helpers
        engine.register_fn(
            "grid",
            |cols: i64, rows: i64, w: f64, h: f64, gap_x: f64, gap_y: f64| -> rhai::Array {
                let mut arr = rhai::Array::new();
                for r in 0..rows {
                    for c in 0..cols {
                        let mut map = rhai::Map::new();
                        map.insert("col".into(), rhai::Dynamic::from(c));
                        map.insert("row".into(), rhai::Dynamic::from(r));
                        map.insert("x".into(), rhai::Dynamic::from(c as f64 * (w + gap_x)));
                        map.insert("y".into(), rhai::Dynamic::from(r as f64 * (h + gap_y)));
                        map.insert("w".into(), rhai::Dynamic::from(w));
                        map.insert("h".into(), rhai::Dynamic::from(h));
                        arr.push(rhai::Dynamic::from(map));
                    }
                }
                arr
            },
        );

        engine.register_fn(
            "radial_repeat",
            |cx: f64, cy: f64, count: i64, radius: f64| -> rhai::Array {
                let mut arr = rhai::Array::new();
                let n = count.max(1);
                for i in 0..n {
                    let angle = (i as f64 / n as f64) * std::f64::consts::TAU;
                    let x = cx + radius * angle.cos();
                    let y = cy + radius * angle.sin();
                    let mut map = rhai::Map::new();
                    map.insert("index".into(), rhai::Dynamic::from(i));
                    map.insert("angle".into(), rhai::Dynamic::from(angle));
                    map.insert("x".into(), rhai::Dynamic::from(x));
                    map.insert("y".into(), rhai::Dynamic::from(y));
                    arr.push(rhai::Dynamic::from(map));
                }
                arr
            },
        );

        engine.register_fn(
            "waveform",
            |x: f64, y: f64, width: f64, height: f64, points: i64, freq: f64| -> rhai::Map {
                let n = points.max(2);
                let mut elements = rhai::Array::new();
                for i in 0..n {
                    let t = i as f64 / (n - 1) as f64;
                    let px = x + t * width;
                    let py = y + (t * freq * std::f64::consts::TAU).sin() * (height / 2.0);
                    let mut el = rhai::Map::new();
                    if i == 0 {
                        el.insert("type".into(), rhai::Dynamic::from("move_to".to_string()));
                    } else {
                        el.insert("type".into(), rhai::Dynamic::from("line_to".to_string()));
                    }
                    el.insert("x".into(), rhai::Dynamic::from(px));
                    el.insert("y".into(), rhai::Dynamic::from(py));
                    elements.push(rhai::Dynamic::from(el));
                }
                let mut obj = rhai::Map::new();
                obj.insert("type".into(), rhai::Dynamic::from("path".to_string()));
                obj.insert("elements".into(), rhai::Dynamic::from(elements));
                obj.insert("closed".into(), rhai::Dynamic::from(false));
                obj
            },
        );

        Self { engine }
    }

    pub fn run_script(
        &self,
        script: &str,
        document: &mut Document,
        _state: &mut AppState,
    ) -> Result<ScriptResult, String> {
        let result_value = self
            .engine
            .eval::<rhai::Map>(script)
            .map_err(|e| format!("Script error: {}", e))?;

        let mut result = ScriptResult::default();

        // Canvas dimensions & name (validated: scripts must not poison the
        // document with NaN/negative sizes)
        if let Some(w) = result_value
            .get("width")
            .and_then(|v| v.clone().try_cast::<f64>())
        {
            if w.is_finite() && w > 0.0 {
                document.width = w.min(16384.0);
            }
        }
        if let Some(h) = result_value
            .get("height")
            .and_then(|v| v.clone().try_cast::<f64>())
        {
            if h.is_finite() && h > 0.0 {
                document.height = h.min(16384.0);
            }
        }
        if let Some(name) = result_value
            .get("name")
            .and_then(|v| v.clone().try_cast::<String>())
        {
            document.name = name;
        }

        // Parse Layers if present
        if let Some(layers_val) = result_value.get("layers") {
            if let Some(layer_arr) = layers_val.clone().try_cast::<rhai::Array>() {
                document.layers.clear();
                for (l_idx, l_item) in layer_arr.into_iter().enumerate() {
                    if let Some(l_map) = l_item.try_cast::<rhai::Map>() {
                        let name = l_map
                            .get("name")
                            .and_then(|v| v.clone().try_cast::<String>())
                            .unwrap_or_else(|| format!("Layer {}", l_idx + 1));
                        let opacity = l_map
                            .get("opacity")
                            .and_then(|v| v.clone().try_cast::<f64>())
                            .map(|v| v as f32)
                            .unwrap_or(1.0);
                        let mut layer = Layer::new(&name);
                        layer.opacity = opacity;

                        if let Some(obj_val) = l_map.get("objects") {
                            if let Some(obj_arr) = obj_val.clone().try_cast::<rhai::Array>() {
                                for item in obj_arr {
                                    if let Some(map) = item.try_cast::<rhai::Map>() {
                                        if let Some(obj) = parse_object_map(&map) {
                                            layer.objects.push(obj);
                                        }
                                    }
                                }
                            }
                        }
                        document.layers.push(layer);
                    }
                }
            }
        } else if let Some(objects) = result_value.get("objects") {
            // Flat objects list added to active layer
            if let Some(arr) = objects.clone().try_cast::<rhai::Array>() {
                for item in arr {
                    if let Some(map) = item.try_cast::<rhai::Map>() {
                        if let Some(obj) = parse_object_map(&map) {
                            result.objects.push(obj.clone());
                            document.add_object(obj);
                        }
                    }
                }
            }
        }

        // Scripts may clear or shrink layers; restore invariants so later
        // operations cannot panic on an empty layer list or stale index.
        document.normalize();

        Ok(result)
    }
}

fn parse_object_map(map: &rhai::Map) -> Option<Object> {
    let obj_type = map
        .get("type")
        .and_then(|v| v.clone().try_cast::<String>())?;
    let x = map
        .get("x")
        .and_then(|v| v.clone().try_cast::<f64>())
        .unwrap_or(0.0);
    let y = map
        .get("y")
        .and_then(|v| v.clone().try_cast::<f64>())
        .unwrap_or(0.0);
    let name = map
        .get("name")
        .and_then(|v| v.clone().try_cast::<String>())
        .unwrap_or_else(|| format!("Object {}", obj_type));

    let mut obj = match obj_type.as_str() {
        "rect" => {
            let w = map
                .get("w")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(100.0);
            let h = map
                .get("h")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(100.0);
            let rx = map
                .get("corner_radius")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(0.0);
            Object::new_rect(&name, x, y, w, h, rx)
        }
        "ellipse" => {
            let rx = map
                .get("rx")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(50.0);
            let ry = map
                .get("ry")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(50.0);
            Object::new_ellipse(&name, x, y, rx, ry)
        }
        "circle" => {
            let r = map
                .get("r")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(50.0);
            Object::new_ellipse(&name, x, y, r, r)
        }
        "line" => {
            let x2 = map
                .get("x2")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(x + 100.0);
            let y2 = map
                .get("y2")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(y);
            Object::new_line(&name, x, y, x2, y2)
        }
        "text" => {
            let text = map
                .get("text")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_default();
            let size = map
                .get("size")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(32.0);
            Object::new_text(&name, &text, x, y, size)
        }
        "clipping_mask" => {
            let mut children = Vec::new();
            if let Some(ch_val) = map.get("children") {
                if let Some(arr) = ch_val.clone().try_cast::<rhai::Array>() {
                    for item in arr {
                        if let Some(ch_map) = item.try_cast::<rhai::Map>() {
                            if let Some(ch_obj) = parse_object_map(&ch_map) {
                                children.push(ch_obj);
                            }
                        }
                    }
                }
            }
            Object {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                object_type: ObjectType::ClippingMask { children },
                transform: Transform {
                    x,
                    y,
                    ..Default::default()
                },
                fill: None,
                stroke: None,
                shadow: None,
                glow: None,
                appearance: Default::default(),
                opacity: 1.0,
                blend_mode: BlendMode::Normal,
                width_profile: None,
                auto_layout: None,
                visible: true,
                locked: false,
            }
        }
        "group" => {
            let mut children = Vec::new();
            if let Some(ch_val) = map.get("children") {
                if let Some(arr) = ch_val.clone().try_cast::<rhai::Array>() {
                    for item in arr {
                        if let Some(ch_map) = item.try_cast::<rhai::Map>() {
                            if let Some(ch_obj) = parse_object_map(&ch_map) {
                                children.push(ch_obj);
                            }
                        }
                    }
                }
            }
            Object {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                object_type: ObjectType::Group(children),
                transform: Transform {
                    x,
                    y,
                    ..Default::default()
                },
                fill: None,
                stroke: None,
                shadow: None,
                glow: None,
                appearance: Default::default(),
                opacity: 1.0,
                blend_mode: BlendMode::Normal,
                width_profile: None,
                auto_layout: None,
                visible: true,
                locked: false,
            }
        }
        "path" => {
            let mut elements = Vec::new();
            if let Some(el_val) = map.get("elements") {
                if let Some(arr) = el_val.clone().try_cast::<rhai::Array>() {
                    for item in arr {
                        if let Some(el_map) = item.try_cast::<rhai::Map>() {
                            let el_type = el_map
                                .get("type")
                                .and_then(|v| v.clone().try_cast::<String>())
                                .unwrap_or_default();
                            match el_type.as_str() {
                                "move_to" => {
                                    let px = el_map
                                        .get("x")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let py = el_map
                                        .get("y")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    elements.push(PathElement::MoveTo(AnchorPoint::new(px, py)));
                                }
                                "line_to" => {
                                    let px = el_map
                                        .get("x")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let py = el_map
                                        .get("y")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    elements.push(PathElement::LineTo(AnchorPoint::new(px, py)));
                                }
                                "curve_to" => {
                                    let c1_x = el_map
                                        .get("c1_x")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let c1_y = el_map
                                        .get("c1_y")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let c2_x = el_map
                                        .get("c2_x")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let c2_y = el_map
                                        .get("c2_y")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let px = el_map
                                        .get("x")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let py = el_map
                                        .get("y")
                                        .and_then(|v| v.clone().try_cast::<f64>())
                                        .unwrap_or(0.0);
                                    let start = match elements.last() {
                                        Some(PathElement::MoveTo(p))
                                        | Some(PathElement::LineTo(p)) => *p,
                                        Some(PathElement::CurveTo(seg)) => seg.end,
                                        _ => AnchorPoint::new(0.0, 0.0),
                                    };
                                    elements.push(PathElement::CurveTo(BezierSegment::cubic(
                                        start,
                                        AnchorPoint::new(c1_x, c1_y),
                                        AnchorPoint::new(c2_x, c2_y),
                                        AnchorPoint::new(px, py),
                                    )));
                                }
                                "close" => {
                                    elements.push(PathElement::ClosePath);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            let closed = map
                .get("closed")
                .and_then(|v| v.clone().try_cast::<bool>())
                .unwrap_or_else(|| elements.iter().any(|e| matches!(e, PathElement::ClosePath)));
            let mut path_data = PathData::new();
            path_data.elements = elements;
            path_data.closed = closed;
            let mut p_obj = Object::new_path(&name, path_data);
            p_obj.transform.x = x;
            p_obj.transform.y = y;
            p_obj
        }
        _ => return None,
    };

    // Styling: Fill
    if let Some(fill_val) = map.get("fill") {
        if let Some(hex_str) = fill_val.clone().try_cast::<String>() {
            if hex_str == "none" || hex_str == "transparent" {
                obj.fill = None;
            } else if let Some(rgba) = parse_svg_color(&hex_str) {
                obj.fill = Some(FillStyle::solid(rgba));
            }
        } else if let Some(f_map) = fill_val.clone().try_cast::<rhai::Map>() {
            let f_type = f_map
                .get("type")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_else(|| "linear".to_string());
            if f_type == "linear" {
                let x1 = f_map
                    .get("x1")
                    .and_then(|v| v.clone().try_cast::<f64>())
                    .unwrap_or(0.0) as f32;
                let y1 = f_map
                    .get("y1")
                    .and_then(|v| v.clone().try_cast::<f64>())
                    .unwrap_or(0.0) as f32;
                let x2 = f_map
                    .get("x2")
                    .and_then(|v| v.clone().try_cast::<f64>())
                    .unwrap_or(0.0) as f32;
                let y2 = f_map
                    .get("y2")
                    .and_then(|v| v.clone().try_cast::<f64>())
                    .unwrap_or(1.0) as f32;
                let mut stops = Vec::new();
                if let Some(s_val) = f_map.get("stops") {
                    if let Some(s_arr) = s_val.clone().try_cast::<rhai::Array>() {
                        for stop_item in s_arr {
                            if let Some(stop_map) = stop_item.try_cast::<rhai::Map>() {
                                let offset = stop_map
                                    .get("offset")
                                    .and_then(|v| v.clone().try_cast::<f64>())
                                    .unwrap_or(0.0)
                                    as f32;
                                let col_str = stop_map
                                    .get("color")
                                    .and_then(|v| v.clone().try_cast::<String>())
                                    .unwrap_or_else(|| "#ffffff".to_string());
                                let col = parse_svg_color(&col_str).unwrap_or([1.0, 1.0, 1.0, 1.0]);
                                stops.push(GradientStop { offset, color: col });
                            }
                        }
                    }
                }
                if stops.is_empty() {
                    stops.push(GradientStop {
                        offset: 0.0,
                        color: [0.0, 0.0, 0.0, 1.0],
                    });
                    stops.push(GradientStop {
                        offset: 1.0,
                        color: [1.0, 1.0, 1.0, 1.0],
                    });
                }
                let base_color = stops[0].color;
                obj.fill = Some(FillStyle {
                    color: base_color,
                    fill_type: FillType::Linear(LinearGradient {
                        start_x: x1,
                        start_y: y1,
                        end_x: x2,
                        end_y: y2,
                        stops,
                    }),
                    rule: FillRule::NonZero,
                    overprint: false,
                    spot: None,
                });
            }
        }
    }

    // Styling: Stroke
    if let Some(stroke_val) = map.get("stroke") {
        if let Some(hex_str) = stroke_val.clone().try_cast::<String>() {
            if hex_str == "none" || hex_str == "transparent" {
                obj.stroke = None;
            } else if let Some(rgba) = parse_svg_color(&hex_str) {
                let w = map
                    .get("stroke_width")
                    .and_then(|v| v.clone().try_cast::<f64>())
                    .unwrap_or(2.0);
                obj.stroke = Some(StrokeStyle {
                    color: rgba,
                    width: w,
                    dash_pattern: None,
                    ..Default::default()
                });
            }
        } else if let Some(s_map) = stroke_val.clone().try_cast::<rhai::Map>() {
            let col_str = s_map
                .get("color")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_else(|| "#000000".to_string());
            let rgba = parse_svg_color(&col_str).unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let w = s_map
                .get("width")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(2.0);
            obj.stroke = Some(StrokeStyle {
                color: rgba,
                width: w,
                dash_pattern: None,
                ..Default::default()
            });
        }
    }

    // Styling: Opacity
    if let Some(op) = map.get("opacity").and_then(|v| v.clone().try_cast::<f64>()) {
        obj.opacity = op.clamp(0.0, 1.0) as f32;
    }

    // Styling: Blend Mode
    if let Some(bm_str) = map
        .get("blend_mode")
        .and_then(|v| v.clone().try_cast::<String>())
    {
        obj.blend_mode = match bm_str.to_lowercase().as_str() {
            "multiply" => BlendMode::Multiply,
            "screen" => BlendMode::Screen,
            "overlay" => BlendMode::Overlay,
            "darken" => BlendMode::Darken,
            "lighten" => BlendMode::Lighten,
            "colordodge" | "color_dodge" => BlendMode::ColorDodge,
            "colorburn" | "color_burn" => BlendMode::ColorBurn,
            "hardlight" | "hard_light" => BlendMode::HardLight,
            "softlight" | "soft_light" => BlendMode::SoftLight,
            "difference" => BlendMode::Difference,
            "exclusion" => BlendMode::Exclusion,
            "hue" => BlendMode::Hue,
            "saturation" => BlendMode::Saturation,
            "color" => BlendMode::Color,
            "luminosity" => BlendMode::Luminosity,
            _ => BlendMode::Normal,
        };
    }

    // Styling: Shadow
    if let Some(sh_val) = map.get("shadow") {
        if let Some(sh_map) = sh_val.clone().try_cast::<rhai::Map>() {
            let sx = sh_map
                .get("offset_x")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(8.0);
            let sy = sh_map
                .get("offset_y")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(12.0);
            let blur = sh_map
                .get("blur")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(16.0);
            let col_str = sh_map
                .get("color")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_else(|| "#000000".to_string());
            let col = parse_svg_color(&col_str).unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let op = sh_map
                .get("opacity")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(0.5) as f32;
            obj.shadow = Some(DropShadow {
                offset_x: sx,
                offset_y: sy,
                blur_radius: blur,
                color: col,
                opacity: op,
            });
        }
    }

    // Styling: Glow
    if let Some(gl_val) = map.get("glow") {
        if let Some(gl_map) = gl_val.clone().try_cast::<rhai::Map>() {
            let rad = gl_map
                .get("radius")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(20.0);
            let col_str = gl_map
                .get("color")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_else(|| "#00ffff".to_string());
            let col = parse_svg_color(&col_str).unwrap_or([0.0, 1.0, 1.0, 1.0]);
            let intensity = gl_map
                .get("intensity")
                .and_then(|v| v.clone().try_cast::<f64>())
                .unwrap_or(0.8) as f32;
            obj.glow = Some(GlowEffect {
                radius: rad,
                color: col,
                intensity,
            });
        }
    }

    Some(obj)
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct ScriptResult {
    pub objects: Vec<Object>,
}
