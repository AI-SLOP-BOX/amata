pub mod object;
pub use object::{BlendMode, Object, ObjectType, Transform};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        };
        doc.layers.push(Layer::new("Layer 1"));
        doc
    }
}

impl Document {
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

    pub fn remove_object(&mut self, id: &str) -> Option<Object> {
        for layer in &mut self.layers {
            if let Some(pos) = layer.objects.iter().position(|o| o.id == id) {
                return Some(layer.objects.remove(pos));
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
