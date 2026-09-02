use crate::core::document::Object;
use crate::core::state::AppState;

/// Points in the app lifecycle where plugins can intercept behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookPoint {
    /// Before rendering an object (can modify or skip)
    BeforeRenderObject,
    /// After rendering an object
    AfterRenderObject,
    /// Before creating an object (can modify or cancel)
    BeforeCreateObject,
    /// After creating an object
    AfterCreateObject,
    /// Before tool action (can override tool behavior)
    BeforeToolAction,
    /// After tool action
    AfterToolAction,
    /// Before file save (can modify what's saved)
    BeforeSave,
    /// After file load
    AfterLoad,
    /// Before SVG export
    BeforeSvgExport,
    /// Custom hook point (plugins can define their own)
    Custom(&'static str),
}

/// Result of a hook invocation
#[allow(clippy::large_enum_variant)]
pub enum HookResult {
    /// Proceed with default behavior
    Continue,
    /// Skip the default behavior
    Skip,
    /// Replace with modified object
    Modify(Object),
    /// Cancel the entire operation
    Cancel,
}

/// A registered hook
pub struct HookRegistration {
    pub point: HookPoint,
    pub plugin_id: String,
    pub priority: i32, // lower = runs first
    #[allow(clippy::type_complexity)]
    pub handler: Box<dyn Fn(&HookPoint, &mut HookContext) -> HookResult + Send + Sync>,
}

pub struct HookContext<'a> {
    pub state: &'a mut AppState,
    pub object: Option<&'a mut Object>,
    pub extra: HookExtra,
}

/// Extra data passed to hooks depending on the hook point
pub enum HookExtra {
    None,
    Render {
        screen_x: f32,
        screen_y: f32,
        zoom: f32,
    },
    Create {
        object_type_name: String,
    },
    Save {
        path: String,
    },
    Load {
        path: String,
    },
    Custom {
        key: String,
        value: String,
    },
}

pub struct HookManager {
    hooks: Vec<HookRegistration>,
}

impl HookManager {
    pub fn new() -> Self {
        Self { hooks: vec![] }
    }

    pub fn register(
        &mut self,
        point: HookPoint,
        plugin_id: String,
        priority: i32,
        handler: impl Fn(&HookPoint, &mut HookContext) -> HookResult + Send + Sync + 'static,
    ) {
        self.hooks.push(HookRegistration {
            point,
            plugin_id,
            priority,
            handler: Box::new(handler),
        });
        self.hooks.sort_by_key(|h| h.priority);
    }

    pub fn unregister(&mut self, plugin_id: &str) {
        self.hooks.retain(|h| h.plugin_id != plugin_id);
    }

    pub fn invoke(
        &self,
        point: &HookPoint,
        ctx: &mut HookContext,
    ) -> HookResult {
        for hook in &self.hooks {
            if &hook.point == point {
                match (hook.handler)(point, ctx) {
                    HookResult::Continue => {}
                    other => return other,
                }
            }
        }
        HookResult::Continue
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}
