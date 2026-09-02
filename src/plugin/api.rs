use std::any::Any;

use crate::core::document::Document;
use crate::core::state::AppState;

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub author: &'static str,
    pub description: &'static str,
}

/// Read-only context passed to plugins during queries
pub struct PluginContext<'a> {
    pub document: &'a Document,
    pub state: &'a AppState,
    pub selected_ids: &'a [String],
}

/// Mutable context passed to plugins during mutations
pub struct PluginContextMut<'a> {
    pub document: &'a mut Document,
    pub state: &'a mut AppState,
    pub selected_ids: &'a [String],
}

/// Core plugin trait. All plugins must implement this.
///
/// Lifecycle:
///   1. `new()` — construct
///   2. `on_load()` — called once when plugin is registered
///   3. `on_activate()` / `on_deactivate()` — toggle
///   4. `on_unload()` — cleanup
///
/// Hooks are called during the relevant app operations.
pub trait Plugin: Any + Send + Sync {
    /// Plugin metadata
    fn info(&self) -> PluginInfo;

    /// Called once after plugin is registered
    fn on_load(&mut self, _ctx: &mut PluginContextMut) {}

    /// Called when plugin is activated
    fn on_activate(&mut self, _ctx: &mut PluginContextMut) {}

    /// Called when plugin is deactivated
    fn on_deactivate(&mut self, _ctx: &mut PluginContextMut) {}

    /// Called before plugin is removed
    fn on_unload(&mut self, _ctx: &mut PluginContextMut) {}

    /// Return custom panels to show in the UI sidebar
    fn ui_panels(&self) -> Vec<PluginPanel> {
        vec![]
    }

    /// Return menu entries to add to the menu bar
    fn menu_entries(&self) -> Vec<PluginMenuEntry> {
        vec![]
    }

    /// Return keyboard shortcuts this plugin registers
    fn shortcuts(&self) -> Vec<PluginShortcut> {
        vec![]
    }

    /// Called when user triggers a custom action from this plugin
    fn on_action(&mut self, _action: &str, _ctx: &mut PluginContextMut) {}

    /// Store/retrieve plugin-specific data (for persistence)
    fn data(&self) -> Option<&dyn Any> {
        None
    }

    fn data_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

/// A UI panel contributed by a plugin
pub struct PluginPanel {
    pub title: String,
    pub render_fn: Box<dyn Fn(&mut AppState) + Send + Sync>,
}

/// A menu entry contributed by a plugin
pub struct PluginMenuEntry {
    pub label: String,
    pub action_id: String,
    pub shortcut: Option<String>,
    pub sub_menu: Option<String>,
}

/// A keyboard shortcut contributed by a plugin
pub struct PluginShortcut {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub action_id: String,
}

/// Helper macro for implementing Plugin
#[macro_export]
macro_rules! impl_plugin {
    ($ty:ty, $id:expr, $name:expr, $ver:expr, $author:expr, $desc:expr) => {
        impl $crate::plugin::Plugin for $ty {
            fn info(&self) -> $crate::plugin::PluginInfo {
                $crate::plugin::PluginInfo {
                    id: $id,
                    name: $name,
                    version: $ver,
                    author: $author,
                    description: $desc,
                }
            }
        }
    };
}
