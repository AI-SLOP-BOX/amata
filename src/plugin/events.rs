use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Events that can be dispatched through the bus
#[derive(Debug, Clone)]
pub enum Event {
    /// Document was modified
    DocumentChanged { object_id: Option<String> },
    /// Object was created
    ObjectCreated { object_id: String },
    /// Object was deleted
    ObjectDeleted { object_id: String },
    /// Object was transformed (moved, scaled, rotated)
    ObjectTransformed { object_id: String },
    /// Selection changed
    SelectionChanged { selected_ids: Vec<String> },
    /// Tool changed
    ToolChanged { tool_name: String },
    /// File was opened
    FileOpened { path: String },
    /// File was saved
    FileSaved { path: String },
    /// Undo/Redo performed
    UndoRedo { is_undo: bool },
    /// Canvas was rendered (after)
    RenderComplete,
    /// Custom event from a plugin
    Custom {
        source: String,
        name: String,
        payload: Option<String>,
    },
}

type EventHandler = Box<dyn Fn(&Event) + Send + Sync>;

pub struct EventBus {
    handlers: HashMap<TypeId, Vec<EventHandler>>,
    global_handlers: Vec<EventHandler>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            global_handlers: Vec::new(),
        }
    }

    /// Subscribe to all events
    pub fn subscribe_all(&mut self, handler: impl Fn(&Event) + Send + Sync + 'static) {
        self.global_handlers.push(Box::new(handler));
    }

    /// Subscribe to a specific event type
    pub fn subscribe(&mut self, handler: impl Fn(&Event) + Send + Sync + 'static) {
        let type_id = TypeId::of::<EventHandler>();
        self.handlers
            .entry(type_id)
            .or_default()
            .push(Box::new(handler));
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: &Event) {
        // Global handlers
        for handler in &self.global_handlers {
            handler(event);
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared event bus wrapped in Arc<RwLock<>>
pub type SharedEventBus = Arc<RwLock<EventBus>>;

pub fn new_shared_event_bus() -> SharedEventBus {
    Arc::new(RwLock::new(EventBus::new()))
}
