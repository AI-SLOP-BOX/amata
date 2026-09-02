use rhai::Engine;

use crate::core::document::{Document, Object};
use crate::core::state::AppState;

pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Simple geometry generation functions (no complex type registration)
        // These return arrays of operations that are applied after execution
        engine.register_fn("rect", |x: f64, y: f64, w: f64, h: f64| -> (f64, f64, f64, f64) {
            (x, y, w, h)
        });
        engine.register_fn("circle", |cx: f64, cy: f64, r: f64| -> (f64, f64, f64, f64) {
            (cx, cy, r, r)
        });
        engine.register_fn("lerp", |a: f64, b: f64, t: f64| -> f64 { a + (b - a) * t });
        engine.register_fn("deg_to_rad", |d: f64| -> f64 { d.to_radians() });
        engine.register_fn("rad_to_deg", |r: f64| -> f64 { r.to_degrees() });
        engine.register_fn("tau", || -> f64 { std::f64::consts::TAU });
        engine.register_fn("pi", || -> f64 { std::f64::consts::PI });

        Self { engine }
    }

    pub fn run_script(
        &self,
        script: &str,
        document: &mut Document,
        _state: &mut AppState,
    ) -> Result<ScriptResult, String> {
        let result_value = self.engine
            .eval::<rhai::Map>(script)
            .map_err(|e| format!("Script error: {}", e))?;

        let mut result = ScriptResult::default();

        // Parse result map for objects
        if let Some(objects) = result_value.get("objects") {
            if let Some(arr) = objects.clone().try_cast::<rhai::Array>() {
                for item in arr {
                    if let Some(map) = item.try_cast::<rhai::Map>() {
                        if let (Some(obj_type), Some(x), Some(y)) = (
                            map.get("type").and_then(|v| v.clone().try_cast::<String>()),
                            map.get("x").and_then(|v| v.clone().try_cast::<f64>()),
                            map.get("y").and_then(|v| v.clone().try_cast::<f64>()),
                        ) {
                            let w = map.get("w").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(100.0);
                            let h = map.get("h").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(100.0);
                            let name = map.get("name").and_then(|v| v.clone().try_cast::<String>()).unwrap_or_else(|| format!("Script {}", obj_type));

                            let obj = match obj_type.as_str() {
                                "rect" => Object::new_rect(&name, x, y, w, h, 0.0),
                                "ellipse" => {
                                    let rx = map.get("rx").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(w * 0.5);
                                    let ry = map.get("ry").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(h * 0.5);
                                    Object::new_ellipse(&name, x, y, rx, ry)
                                }
                                "circle" => {
                                    let r = map.get("r").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(50.0);
                                    Object::new_ellipse(&name, x, y, r, r)
                                }
                                "line" => {
                                    let x2 = map.get("x2").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(x + 100.0);
                                    let y2 = map.get("y2").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(y);
                                    Object::new_line(&name, x, y, x2, y2)
                                }
                                "text" => {
                                    let text = map.get("text").and_then(|v| v.clone().try_cast::<String>()).unwrap_or_default();
                                    let size = map.get("size").and_then(|v| v.clone().try_cast::<f64>()).unwrap_or(32.0);
                                    Object::new_text(&name, &text, x, y, size)
                                }
                                _ => continue,
                            };
                            result.objects.push(obj);
                        }
                    }
                }
            }
        }

        // Apply created objects to document
        for obj in &result.objects {
            document.add_object(obj.clone());
        }

        Ok(result)
    }
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
