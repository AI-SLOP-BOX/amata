use super::document::Document;

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
    pub layer_idx: usize,
    pub position: usize,
}

impl RemoveObjectCommand {
    pub fn new(obj: super::document::Object, layer_idx: usize, position: usize) -> Self {
        Self {
            object: Some(obj),
            layer_idx,
            position,
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
