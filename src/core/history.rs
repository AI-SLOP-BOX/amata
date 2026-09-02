use super::document::Document;

pub trait Command {
    fn execute(&self, doc: &mut Document);
    fn undo(&self, doc: &mut Document);
    fn name(&self) -> &str;
}

pub struct UndoManager {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_steps: usize,
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
        }
    }

    pub fn execute(&mut self, cmd: Box<dyn Command>, doc: &mut Document) {
        cmd.execute(doc);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_steps {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, doc: &mut Document) -> Option<&str> {
        if let Some(cmd) = self.undo_stack.pop() {
            let name = cmd.name().to_string();
            cmd.undo(doc);
            self.redo_stack.push(cmd);
            Some(Box::leak(name.into_boxed_str()))
        } else {
            None
        }
    }

    pub fn redo(&mut self, doc: &mut Document) -> Option<&str> {
        if let Some(cmd) = self.redo_stack.pop() {
            let name = cmd.name().to_string();
            cmd.execute(doc);
            self.undo_stack.push(cmd);
            Some(Box::leak(name.into_boxed_str()))
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
        self.undo_stack.last().map(|c| c.name())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.name())
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn undo_stack(&self) -> &[Box<dyn Command>] {
        &self.undo_stack
    }

    pub fn redo_stack(&self) -> &[Box<dyn Command>] {
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
        for obj in doc.all_objects_mut().map(|(_, o)| o) {
            if obj.id == self.object_id {
                obj.transform.x = self.new_x;
                obj.transform.y = self.new_y;
                break;
            }
        }
    }

    fn undo(&self, doc: &mut Document) {
        for obj in doc.all_objects_mut().map(|(_, o)| o) {
            if obj.id == self.object_id {
                obj.transform.x = self.old_x;
                obj.transform.y = self.old_y;
                break;
            }
        }
    }

    fn name(&self) -> &str {
        "Move Object"
    }
}
