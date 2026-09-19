use super::document::{Document, Layer, Transform};

pub trait Command {
    fn execute(&self, doc: &mut Document);
    fn undo(&self, doc: &mut Document);
    fn name(&self) -> &str;
}

pub struct UndoStep {
    pub cmd: Box<dyn Command>,
    pub state_id_before: u64,
    pub state_id_after: u64,
}

impl std::ops::Deref for UndoStep {
    type Target = dyn Command;
    fn deref(&self) -> &Self::Target {
        &*self.cmd
    }
}

pub struct UndoManager {
    undo_stack: Vec<UndoStep>,
    redo_stack: Vec<UndoStep>,
    max_steps: usize,
    state_counter: u64,
    current_state_id: u64,
    saved_state_id: Option<u64>,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_steps: 100,
            state_counter: 0,
            current_state_id: 0,
            saved_state_id: Some(0),
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self.saved_state_id {
            Some(saved) => saved != self.current_state_id,
            None => true,
        }
    }

    pub fn mark_saved(&mut self) {
        self.saved_state_id = Some(self.current_state_id);
    }

    pub fn mark_dirty(&mut self) {
        self.saved_state_id = None;
    }

    pub fn execute(&mut self, cmd: Box<dyn Command>, doc: &mut Document) {
        cmd.execute(doc);
        self.state_counter += 1;
        let step = UndoStep {
            cmd,
            state_id_before: self.current_state_id,
            state_id_after: self.state_counter,
        };
        self.current_state_id = self.state_counter;
        self.undo_stack.push(step);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_steps {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, doc: &mut Document) -> Option<String> {
        if let Some(step) = self.undo_stack.pop() {
            let name = step.cmd.name().to_string();
            step.cmd.undo(doc);
            self.current_state_id = step.state_id_before;
            self.redo_stack.push(step);
            Some(name)
        } else {
            None
        }
    }

    pub fn redo(&mut self, doc: &mut Document) -> Option<String> {
        if let Some(step) = self.redo_stack.pop() {
            let name = step.cmd.name().to_string();
            step.cmd.execute(doc);
            self.current_state_id = step.state_id_after;
            self.undo_stack.push(step);
            Some(name)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.cmd.name())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.cmd.name())
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.state_counter += 1;
        self.current_state_id = self.state_counter;
        self.saved_state_id = Some(self.current_state_id);
    }

    pub fn undo_stack(&self) -> &[UndoStep] {
        &self.undo_stack
    }

    pub fn redo_stack(&self) -> &[UndoStep] {
        &self.redo_stack
    }
}

// --- Concrete commands ---

pub struct AddObjectCommand {
    pub object: Option<super::document::Object>,
    pub object_id: String,
}

impl AddObjectCommand {
    pub fn new(obj: super::document::Object) -> Self {
        let id = obj.id.clone();
        Self {
            object: Some(obj),
            object_id: id,
        }
    }
}

impl Command for AddObjectCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(obj) = &self.object {
            doc.add_object(obj.clone());
        }
    }

    fn undo(&self, doc: &mut Document) {
        doc.remove_object(&self.object_id);
    }

    fn name(&self) -> &str {
        "Add Object"
    }
}

pub struct RemoveObjectCommand {
    pub object: Option<super::document::Object>,
    pub parent: Option<String>,
    pub layer_idx: usize,
    pub position: usize,
}

impl RemoveObjectCommand {
    pub fn new(obj: super::document::Object, layer_idx: usize, position: usize) -> Self {
        Self {
            object: Some(obj),
            parent: None,
            layer_idx,
            position,
        }
    }

    pub fn new_nested(
        obj: super::document::Object,
        parent: String,
        layer_idx: usize,
        position: usize,
    ) -> Self {
        Self {
            object: Some(obj),
            parent: Some(parent),
            layer_idx,
            position,
        }
    }

    /// Capture location via [`Document::parent_of`]; nested children
    /// restore into their parent group instead of the layer top level.
    pub fn located(obj: super::document::Object, doc: &Document) -> Self {
        let id = obj.id.clone();
        match doc.parent_of(&id) {
            Some((Some(pid), li, pos)) => Self::new_nested(obj, pid, li, pos),
            Some((None, li, pos)) => Self::new(obj, li, pos),
            None => Self::new(obj, 0, 0),
        }
    }
}

impl Command for RemoveObjectCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(obj) = &self.object {
            doc.remove_object(&obj.id);
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(obj) = &self.object {
            if let Some(ref pid) = self.parent {
                if let Some(parent) = doc.find_object_mut(pid) {
                    match &mut parent.object_type {
                        super::document::ObjectType::Group(children)
                        | super::document::ObjectType::ClippingMask { children } => {
                            let pos = self.position.min(children.len());
                            children.insert(pos, obj.clone());
                            return;
                        }
                        _ => {}
                    }
                }
            }
            if let Some(layer) = doc.layers.get_mut(self.layer_idx) {
                let pos = self.position.min(layer.objects.len());
                layer.objects.insert(pos, obj.clone());
            }
        }
    }

    fn name(&self) -> &str {
        "Remove Object"
    }
}

pub struct MoveObjectCommand {
    pub object_id: String,
    pub old_x: f64,
    pub old_y: f64,
    pub new_x: f64,
    pub new_y: f64,
}

impl Command for MoveObjectCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            obj.transform.x = self.new_x;
            obj.transform.y = self.new_y;
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            obj.transform.x = self.old_x;
            obj.transform.y = self.old_y;
        }
    }

    fn name(&self) -> &str {
        "Move Object"
    }
}

/// Whole-transform edit (panel widgets, align tools). Recorded once per
/// gesture so dragging a value does not flood the undo stack.
pub struct TransformCommand {
    pub object_id: String,
    pub old_t: Transform,
    pub new_t: Transform,
}

impl Command for TransformCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            obj.transform = self.new_t.clone();
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            obj.transform = self.old_t.clone();
        }
    }

    fn name(&self) -> &str {
        "Edit Transform"
    }
}

/// Whole-object edit for panel widgets that touch non-transform fields
/// (opacity, stroke width, fill, typography…). Coalesced per gesture like
/// TransformCommand.
pub struct ObjectCommand {
    pub object_id: String,
    pub old_obj: super::document::Object,
    pub new_obj: super::document::Object,
}

impl Command for ObjectCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            *obj = self.new_obj.clone();
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            *obj = self.old_obj.clone();
        }
    }

    fn name(&self) -> &str {
        "Edit Object"
    }
}

fn clamp_active(doc: &mut Document, idx: usize) {
    doc.active_layer_idx = idx.min(doc.layers.len().saturating_sub(1));
}

/// Layer creation (add / duplicate). Tracks the previous active layer so
/// Undo restores the exact prior state.
pub struct AddLayerCommand {
    pub layer: Layer,
    pub index: usize,
    pub prev_active: usize,
}

impl Command for AddLayerCommand {
    fn execute(&self, doc: &mut Document) {
        let pos = self.index.min(doc.layers.len());
        // Avoid duplicating on redo-after-undo races: replace if present.
        if let Some(existing) = doc.layers.iter().position(|l| l.id == self.layer.id) {
            doc.layers.remove(existing);
        }
        let pos = pos.min(doc.layers.len());
        doc.layers.insert(pos, self.layer.clone());
        clamp_active(doc, pos);
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(pos) = doc.layers.iter().position(|l| l.id == self.layer.id) {
            doc.layers.remove(pos);
        }
        clamp_active(doc, self.prev_active);
    }

    fn name(&self) -> &str {
        "Add Layer"
    }
}

/// Layer deletion. Undo reinserts at the original index.
pub struct RemoveLayerCommand {
    pub layer: Layer,
    pub index: usize,
}

impl Command for RemoveLayerCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(pos) = doc.layers.iter().position(|l| l.id == self.layer.id) {
            doc.layers.remove(pos);
        }
        clamp_active(doc, doc.active_layer_idx);
    }

    fn undo(&self, doc: &mut Document) {
        let pos = self.index.min(doc.layers.len());
        if !doc.layers.iter().any(|l| l.id == self.layer.id) {
            doc.layers.insert(pos, self.layer.clone());
        }
        clamp_active(doc, pos);
    }

    fn name(&self) -> &str {
        "Delete Layer"
    }
}

fn apply_id_order<T, F>(items: &mut Vec<T>, order: &[String], id_of: F)
where
    F: Fn(&T) -> &str,
{
    let mut ranked = std::collections::HashMap::new();
    for (rank, id) in order.iter().enumerate() {
        ranked.insert(id.clone(), rank);
    }
    let mut counter = order.len();
    let mut fallback = std::collections::HashMap::new();
    for item in items.iter() {
        let key = id_of(item).to_string();
        if !ranked.contains_key(&key) {
            fallback.insert(key, counter);
            counter += 1;
        }
    }
    items.sort_by_key(|item| {
        let key = id_of(item).to_string();
        // NOTE: must be lazy — eager `unwrap_or` would index `fallback`
        // even for keys present in `ranked` and panic.
        if let Some(&rank) = ranked.get(&key) {
            rank
        } else {
            *fallback.get(&key).unwrap_or(&usize::MAX)
        }
    });
}

/// Layer z-order change (move up/down).
pub struct ReorderLayersCommand {
    pub old_order: Vec<String>,
    pub new_order: Vec<String>,
}

impl Command for ReorderLayersCommand {
    fn execute(&self, doc: &mut Document) {
        apply_id_order(&mut doc.layers, &self.new_order, |l| &l.id);
    }

    fn undo(&self, doc: &mut Document) {
        apply_id_order(&mut doc.layers, &self.old_order, |l| &l.id);
    }

    fn name(&self) -> &str {
        "Reorder Layers"
    }
}

/// Object z-order change within one layer (move up/down).
pub struct ReorderObjectsCommand {
    pub layer_id: String,
    pub old_order: Vec<String>,
    pub new_order: Vec<String>,
}

impl Command for ReorderObjectsCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(layer) = doc.layers.iter_mut().find(|l| l.id == self.layer_id) {
            apply_id_order(&mut layer.objects, &self.new_order, |o| &o.id);
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(layer) = doc.layers.iter_mut().find(|l| l.id == self.layer_id) {
            apply_id_order(&mut layer.objects, &self.old_order, |o| &o.id);
        }
    }

    fn name(&self) -> &str {
        "Reorder Objects"
    }
}

/// Whole-layer edit (currently opacity). Layers live outside objects,
/// so they get their own command type with the same coalescing pattern.
pub struct LayerCommand {
    pub layer_id: String,
    pub old_layer: super::document::Layer,
    pub new_layer: super::document::Layer,
}

impl Command for LayerCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(layer) = doc.layers.iter_mut().find(|l| l.id == self.layer_id) {
            *layer = self.new_layer.clone();
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(layer) = doc.layers.iter_mut().find(|l| l.id == self.layer_id) {
            *layer = self.old_layer.clone();
        }
    }

    fn name(&self) -> &str {
        "Edit Layer"
    }
}

pub struct BatchCommand {
    pub name: String,
    pub commands: Vec<Box<dyn Command>>,
}

impl BatchCommand {
    pub fn new(name: impl Into<String>, commands: Vec<Box<dyn Command>>) -> Self {
        Self {
            name: name.into(),
            commands,
        }
    }
}

impl Command for BatchCommand {
    fn execute(&self, doc: &mut Document) {
        for cmd in &self.commands {
            cmd.execute(doc);
        }
    }

    fn undo(&self, doc: &mut Document) {
        for cmd in self.commands.iter().rev() {
            cmd.undo(doc);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub struct ModifyPathCommand {
    pub object_id: String,
    pub old_elements: Vec<crate::core::path::PathElement>,
    pub new_elements: Vec<crate::core::path::PathElement>,
}

impl ModifyPathCommand {
    pub fn new(
        object_id: impl Into<String>,
        old_elements: Vec<crate::core::path::PathElement>,
        new_elements: Vec<crate::core::path::PathElement>,
    ) -> Self {
        Self {
            object_id: object_id.into(),
            old_elements,
            new_elements,
        }
    }
}

impl Command for ModifyPathCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            if let crate::core::document::ObjectType::Path(ref mut p) = obj.object_type {
                p.elements = self.new_elements.clone();
            }
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            if let crate::core::document::ObjectType::Path(ref mut p) = obj.object_type {
                p.elements = self.old_elements.clone();
            }
        }
    }

    fn name(&self) -> &str {
        "Edit Path Nodes"
    }
}

pub struct ModifyTextCommand {
    pub object_id: String,
    pub old_text: String,
    pub old_style: crate::core::document::TextStyle,
    pub new_text: String,
    pub new_style: crate::core::document::TextStyle,
}

impl ModifyTextCommand {
    pub fn new(
        object_id: impl Into<String>,
        old_text: String,
        old_style: crate::core::document::TextStyle,
        new_text: String,
        new_style: crate::core::document::TextStyle,
    ) -> Self {
        Self {
            object_id: object_id.into(),
            old_text,
            old_style,
            new_text,
            new_style,
        }
    }
}

impl Command for ModifyTextCommand {
    fn execute(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            obj.object_type = crate::core::document::ObjectType::Text {
                text: self.new_text.clone(),
                font_size: self.new_style.font_size,
                style: self.new_style.clone(),
            };
        }
    }

    fn undo(&self, doc: &mut Document) {
        if let Some(obj) = doc.find_object_mut(&self.object_id) {
            obj.object_type = crate::core::document::ObjectType::Text {
                text: self.old_text.clone(),
                font_size: self.old_style.font_size,
                style: self.old_style.clone(),
            };
        }
    }

    fn name(&self) -> &str {
        "Change Typography"
    }
}
