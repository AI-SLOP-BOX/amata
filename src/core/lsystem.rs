use super::path::{PathData, StrokeStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LSystemPreset {
    Tree,
    Dragon,
    Snowflake,
    Hilbert,
}

pub fn generate_lsystem(
    preset: LSystemPreset,
    iterations: usize,
    cx: f64,
    cy: f64,
    step_size: f64,
) -> PathData {
    let (axiom, rules, angle_deg, start_angle): (&str, HashMap<char, String>, f64, f64) =
        match preset {
            LSystemPreset::Tree => {
                let mut r = HashMap::new();
                r.insert('X', "F+[[X]-X]-F[-FX]+X".to_string());
                r.insert('F', "FF".to_string());
                ("X", r, 25.0, -90.0)
            }
            LSystemPreset::Dragon => {
                let mut r = HashMap::new();
                r.insert('X', "X+YF+".to_string());
                r.insert('Y', "-FX-Y".to_string());
                ("FX", r, 90.0, 0.0)
            }
            LSystemPreset::Snowflake => {
                let mut r = HashMap::new();
                r.insert('F', "F+F--F+F".to_string());
                ("F--F--F", r, 60.0, 0.0)
            }
            LSystemPreset::Hilbert => {
                let mut r = HashMap::new();
                r.insert('A', "-BF+AFA+FB-".to_string());
                r.insert('B', "+AF-BFB-FA+".to_string());
                ("A", r, 90.0, 0.0)
            }
        };

    // Expand L-System string
    let mut current = axiom.to_string();
    let iters = iterations.min(6); // Prevent exponential blowup
    for _ in 0..iters {
        let mut next = String::with_capacity(current.len() * 3);
        for ch in current.chars() {
            if let Some(replacement) = rules.get(&ch) {
                next.push_str(replacement);
            } else {
                next.push(ch);
            }
        }
        current = next;
    }

    // Turtle graphics interpretation
    let mut path = PathData::new();
    let mut x = cx;
    let mut y = cy;
    let mut heading = start_angle.to_radians();
    let angle_rad = angle_deg.to_radians();

    path.push_move_to(x, y);

    let mut stack: Vec<(f64, f64, f64)> = Vec::new();

    for ch in current.chars() {
        match ch {
            'F' | 'G' => {
                x += heading.cos() * step_size;
                y += heading.sin() * step_size;
                path.push_line_to(x, y);
            }
            '+' => heading += angle_rad,
            '-' => heading -= angle_rad,
            '[' => stack.push((x, y, heading)),
            ']' => {
                if let Some((sx, sy, sh)) = stack.pop() {
                    x = sx;
                    y = sy;
                    heading = sh;
                    path.push_move_to(x, y);
                }
            }
            _ => {}
        }
    }

    path.fill = None;
    path.stroke = Some(StrokeStyle {
        color: match preset {
            LSystemPreset::Tree => [0.1, 0.7, 0.3, 1.0],
            LSystemPreset::Dragon => [0.95, 0.2, 0.3, 1.0],
            LSystemPreset::Snowflake => [0.2, 0.8, 1.0, 1.0],
            LSystemPreset::Hilbert => [0.8, 0.3, 0.95, 1.0],
        },
        width: 1.5,
        dash_pattern: None,
        ..StrokeStyle::default()
    });

    path
}
