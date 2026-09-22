pub mod object;
pub use object::{
    BlendMode, FontStyle, Object, ObjectType, TextAnchor, TextArea, TextStyle, Transform,
    VariationSetting, char_advance_estimate, layout_text,
};
#[allow(unused_imports)]
pub use object::compute_wrapped_lines;

/// Canvas guide (moved here from AppState so guides persist with the
/// document instead of evaporating on every save/reload).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuideOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub orientation: GuideOrientation,
    pub position: f64,
}

impl Default for Guide {
    fn default() -> Self {
        Self {
            orientation: GuideOrientation::Horizontal,
            position: 0.0,
        }
    }
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single artboard within a multi-artboard document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artboard {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Figma-style layout grid (columns / rows / square grid).
    #[serde(default)]
    pub layout_grid: Option<crate::core::layout_grid::LayoutGrid>,
}

impl Artboard {
    pub fn new(name: &str, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            x,
            y,
            width,
            height,
            layout_grid: None,
        }
    }
}

/// Color mode of the document. Affects which color picker is primary and
/// how SVG export communicates color space (CMYK values are exported as
/// ICC-based `<color-profile>` elements).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ColorMode {
    #[default]
    Rgb,
    Cmyk,
}


impl std::fmt::Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rgb => write!(f, "RGB"),
            Self::Cmyk => write!(f, "CMYK"),
        }
    }
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            objects: Vec::new(),
            visible: true,
            locked: false,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub objects: Vec<Object>,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub name: String,
    pub layers: Vec<Layer>,
    pub active_layer_idx: usize,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub symbols: Vec<Symbol>,
    /// Timeline animation (persisted; previously AppState-only, so every
    /// animation evaporated on save/reload).
    #[serde(default)]
    pub timeline: super::timeline::Timeline,
    /// Canvas guides (persisted for the same reason).
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Color mode of the document (RGB or CMYK). Affects the primary color
    /// picker shown in the style panel and how colors are serialized.
    #[serde(default)]
    pub color_mode: ColorMode,
    /// Spot color library (print separations).
    #[serde(default)]
    pub spots: Vec<super::print::SpotColor>,
    /// Bleed in points (3mm ≈ 8.5pt is the Japanese offset standard).
    #[serde(default)]
    pub bleed: f64,
    /// Perspective guide set (`None` = never set up).
    #[serde(default)]
    pub perspective: Option<super::perspective::PerspectiveGrid>,
    /// Artboards owned by this document. Empty means a single implicit
    /// artboard using `width`/`height`.
    #[serde(default)]
    pub artboards: Vec<Artboard>,
}

impl Default for Document {
    fn default() -> Self {
        let mut doc = Self {
            name: "Untitled".to_string(),
            layers: Vec::new(),
            active_layer_idx: 0,
            width: 1920.0,
            height: 1080.0,
            symbols: Vec::new(),
            timeline: super::timeline::Timeline::default(),
            guides: Vec::new(),
            color_mode: ColorMode::Rgb,
            spots: super::print::default_spots(),
            bleed: 0.0,
            perspective: None,
            artboards: Vec::new(),
        };
        doc.layers.push(Layer::new("Layer 1"));
        doc
    }
}

impl Document {
    /// Repair invariants that untrusted inputs (project JSON, scripts) may break:
    /// at least one layer exists, the active index is in range, and canvas
    /// dimensions are finite and positive.
    pub fn normalize(&mut self) {
        if self.layers.is_empty() {
            self.layers.push(Layer::new("Layer 1"));
        }
        if self.active_layer_idx >= self.layers.len() {
            self.active_layer_idx = self.layers.len() - 1;
        }
        if !self.width.is_finite() || self.width <= 0.0 {
            self.width = 1920.0;
        }
        if !self.height.is_finite() || self.height <= 0.0 {
            self.height = 1080.0;
        }
    }

    /// Return the effective artboard list. If `artboards` is empty, returns
    /// a single implicit artboard derived from `width`/`height`.
    pub fn effective_artboards(&self) -> Vec<Artboard> {
        if self.artboards.is_empty() {
            vec![Artboard::new("Artboard 1", 0.0, 0.0, self.width, self.height)]
        } else {
            self.artboards.clone()
        }
    }

    /// Return the artboard dimensions for a given index, falling back to
    /// the document size if the index is out of range.
    pub fn artboard_rect(&self, idx: usize) -> (f64, f64, f64, f64) {
        if let Some(ab) = self.artboards.get(idx) {
            (ab.x, ab.y, ab.width, ab.height)
        } else {
            (0.0, 0.0, self.width, self.height)
        }
    }

    pub fn active_layer(&self) -> &Layer {
        &self.layers[self.active_layer_idx]
    }

    pub fn active_layer_mut(&mut self) -> &mut Layer {
        &mut self.layers[self.active_layer_idx]
    }

    pub fn add_object(&mut self, obj: Object) {
        self.active_layer_mut().objects.push(obj);
    }

    pub fn all_objects(&self) -> impl DoubleEndedIterator<Item = (usize, &Object)> {
        self.layers
            .iter()
            .enumerate()
            .flat_map(|(i, layer)| layer.objects.iter().map(move |obj| (i, obj)))
    }

    pub fn all_objects_mut(&mut self) -> impl Iterator<Item = (usize, &mut Object)> {
        self.layers
            .iter_mut()
            .enumerate()
            .flat_map(|(i, layer)| layer.objects.iter_mut().map(move |obj| (i, obj)))
    }

    pub fn object_by_id(&self, id: &str) -> Option<(usize, &Object)> {
        self.all_objects().find(|(_, o)| o.id == id)
    }

    /// Deep lookup that also descends into Group / ClippingMask children.
    /// Flat `all_objects` misses nested children, so id-addressed operations
    /// (undo commands, timeline tracks) must use this.
    pub fn find_object(&self, id: &str) -> Option<&Object> {
        for layer in &self.layers {
            if let Some(o) = find_in_objects(&layer.objects, id) {
                return Some(o);
            }
        }
        None
    }

    pub fn find_object_mut(&mut self, id: &str) -> Option<&mut Object> {
        for layer in &mut self.layers {
            if let Some(o) = find_in_objects_mut(&mut layer.objects, id) {
                return Some(o);
            }
        }
        None
    }

    /// Locate an object: (parent object id or None for top-level,
    /// layer index, index within the parent or layer).
    pub fn parent_of(&self, id: &str) -> Option<(Option<String>, usize, usize)> {
        for (li, layer) in self.layers.iter().enumerate() {
            if let Some(pos) = layer.objects.iter().position(|o| o.id == id) {
                return Some((None, li, pos));
            }
            if let Some((pid, pos)) = parent_in_objects(&layer.objects, id) {
                return Some((Some(pid), li, pos));
            }
        }
        None
    }

    pub fn remove_object(&mut self, id: &str) -> Option<Object> {
        for layer in &mut self.layers {
            if let Some(pos) = layer.objects.iter().position(|o| o.id == id) {
                return Some(layer.objects.remove(pos));
            }
            if let Some(o) = remove_from_objects(&mut layer.objects, id) {
                return Some(o);
            }
        }
        None
    }

    pub fn symbol_by_id(&self, id: &str) -> Option<&Symbol> {
        let clean_id = id.trim_start_matches('#');
        self.symbols.iter().find(|s| s.id == clean_id)
    }

    pub fn symbol_by_id_mut(&mut self, id: &str) -> Option<&mut Symbol> {
        let clean_id = id.trim_start_matches('#');
        self.symbols.iter_mut().find(|s| s.id == clean_id)
    }

    pub fn add_symbol(&mut self, symbol: Symbol) {
        if let Some(pos) = self.symbols.iter().position(|s| s.id == symbol.id) {
            self.symbols[pos] = symbol;
        } else {
            self.symbols.push(symbol);
        }
    }

    pub fn create_component_from_selection(&mut self, ids: &[String]) -> Option<String> {
        if ids.is_empty() {
            return None;
        }
        let mut target_objects = Vec::new();
        for id in ids {
            if let Some(obj) = self.remove_object(id) {
                target_objects.push(obj);
            }
        }
        if target_objects.is_empty() {
            return None;
        }

        let symbol_id = format!("comp_{}", &Uuid::new_v4().to_string()[..8]);
        let master_obj = if target_objects.len() == 1 {
            target_objects.remove(0)
        } else {
            Object::new_group("Component Master", target_objects)
        };
        let symbol_name = master_obj.name.clone();
        let sym = Symbol {
            id: symbol_id.clone(),
            name: symbol_name.clone(),
            object: master_obj,
            use_count: 1,
        };
        self.add_symbol(sym);

        let instance =
            Object::new_use(&symbol_name, &format!("#{symbol_id}"), 0.0, 0.0, None, None);
        self.add_object(instance);
        Some(symbol_id)
    }

    pub fn instantiate_component(&mut self, symbol_id: &str, x: f64, y: f64) -> Option<Object> {
        let sym = self.symbol_by_id(symbol_id)?;
        let name = sym.name.clone();
        let clean_id = sym.id.clone();
        Some(Object::new_use(
            &format!("{name} Instance"),
            &format!("#{clean_id}"),
            x,
            y,
            None,
            None,
        ))
    }

    pub fn move_layer_up(&mut self, idx: usize) {
        if idx + 1 < self.layers.len() {
            self.layers.swap(idx, idx + 1);
            if self.active_layer_idx == idx {
                self.active_layer_idx = idx + 1;
            } else if self.active_layer_idx == idx + 1 {
                self.active_layer_idx = idx;
            }
        }
    }

    pub fn move_layer_down(&mut self, idx: usize) {
        if idx > 0 && idx < self.layers.len() {
            self.layers.swap(idx, idx - 1);
            if self.active_layer_idx == idx {
                self.active_layer_idx = idx - 1;
            } else if self.active_layer_idx == idx - 1 {
                self.active_layer_idx = idx;
            }
        }
    }

    pub fn move_object_up(&mut self, layer_idx: usize, obj_idx: usize) {
        if layer_idx < self.layers.len() && obj_idx + 1 < self.layers[layer_idx].objects.len() {
            self.layers[layer_idx].objects.swap(obj_idx, obj_idx + 1);
        }
    }

    pub fn move_object_down(&mut self, layer_idx: usize, obj_idx: usize) {
        if layer_idx < self.layers.len()
            && obj_idx > 0
            && obj_idx < self.layers[layer_idx].objects.len()
        {
            self.layers[layer_idx].objects.swap(obj_idx, obj_idx - 1);
        }
    }

    pub fn move_object_to_layer(&mut self, from_layer: usize, obj_idx: usize, to_layer: usize) {
        if from_layer < self.layers.len()
            && to_layer < self.layers.len()
            && obj_idx < self.layers[from_layer].objects.len()
        {
            let obj = self.layers[from_layer].objects.remove(obj_idx);
            self.layers[to_layer].objects.push(obj);
        }
    }
}

/// Maximum recursion depth when descending into Group / ClippingMask
/// children.  Guards against pathological (or corrupted) documents where a
/// group transitively contains itself, which would otherwise recurse until
/// stack overflow.
const MAX_NESTING_DEPTH: usize = 128;

fn find_in_objects<'a>(objs: &'a [Object], id: &str) -> Option<&'a Object> {
    find_in_objects_depth(objs, id, 0)
}

fn find_in_objects_depth<'a>(objs: &'a [Object], id: &str, depth: usize) -> Option<&'a Object> {
    if depth > MAX_NESTING_DEPTH {
        return None;
    }
    for o in objs {
        if o.id == id {
            return Some(o);
        }
        match &o.object_type {
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                if let Some(found) = find_in_objects_depth(children, id, depth + 1) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn parent_in_objects(objs: &[Object], id: &str) -> Option<(String, usize)> {
    parent_in_objects_depth(objs, id, 0)
}

fn parent_in_objects_depth(
    objs: &[Object],
    id: &str,
    depth: usize,
) -> Option<(String, usize)> {
    if depth > MAX_NESTING_DEPTH {
        return None;
    }
    for o in objs {
        match &o.object_type {
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                if let Some(pos) = children.iter().position(|c| c.id == id) {
                    return Some((o.id.clone(), pos));
                }
                if let Some(found) = parent_in_objects_depth(children, id, depth + 1) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_in_objects_mut<'a>(objs: &'a mut [Object], id: &str) -> Option<&'a mut Object> {
    find_in_objects_mut_depth(objs, id, 0)
}

fn find_in_objects_mut_depth<'a>(
    objs: &'a mut [Object],
    id: &str,
    depth: usize,
) -> Option<&'a mut Object> {
    if depth > MAX_NESTING_DEPTH {
        return None;
    }
    for o in objs.iter_mut() {
        if o.id == id {
            return Some(o);
        }
        match &mut o.object_type {
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                if let Some(found) = find_in_objects_mut_depth(children, id, depth + 1) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

#[allow(clippy::ptr_arg)]
fn remove_from_objects(objs: &mut Vec<Object>, id: &str) -> Option<Object> {
    for o in objs.iter_mut() {
        match &mut o.object_type {
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                if let Some(pos) = children.iter().position(|c| c.id == id) {
                    return Some(children.remove(pos));
                }
                if let Some(found) = remove_from_objects(children, id) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════
// Symbol: Reusable object definition
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub object: Object,
    pub use_count: usize,
}

impl Symbol {
    pub fn new(name: &str, object: Object) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object,
            use_count: 0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Width Point: Variable stroke width
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidthPoint {
    pub position: f64,
    pub width: f64,
    pub side: WidthSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WidthSide {
    Left,
    Right,
    #[default]
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidthProfile {
    pub points: Vec<WidthPoint>,
}

impl Default for WidthProfile {
    fn default() -> Self {
        Self {
            points: vec![
                WidthPoint {
                    position: 0.0,
                    width: 1.0,
                    side: WidthSide::Both,
                },
                WidthPoint {
                    position: 1.0,
                    width: 1.0,
                    side: WidthSide::Both,
                },
            ],
        }
    }
}

#[cfg(test)]
mod nesting_guard_tests {
    use super::*;

    fn deep_chain(depth: usize) -> Object {
        let mut obj = Object::new_rect("leaf", 0.0, 0.0, 10.0, 10.0, 0.0);
        for i in 0..depth {
            obj = Object::new_group(&format!("g{i}"), vec![obj]);
        }
        obj
    }

    #[test]
    fn deep_nesting_lookup_does_not_overflow() {
        let mut doc = Document::default();
        // Far beyond MAX_NESTING_DEPTH: must return gracefully, not overflow.
        doc.add_object(deep_chain(2000));
        assert!(doc.find_object("missing").is_none());
    }

    #[test]
    fn normal_nesting_lookup_still_works() {
        let mut doc = Document::default();
        let inner = Object::new_rect("inner", 0.0, 0.0, 10.0, 10.0, 0.0);
        let inner_id = inner.id.clone();
        let g2 = Object::new_group("g2", vec![inner.clone()]);
        let g2_id = g2.id.clone();
        doc.add_object(Object::new_group("g", vec![g2]));
        assert!(doc.find_object(&inner_id).is_some());
        assert!(doc.find_object_mut(&inner_id).is_some());
        let parent = doc.parent_of(&inner_id).and_then(|(p, _, _)| p);
        assert_eq!(parent.as_deref(), Some(g2_id.as_str()));
    }
}
