use super::api::{Plugin, PluginContextMut, PluginInfo};
use super::events::{Event, SharedEventBus};
use super::hooks::HookManager;
use crate::core::document::Document;
use crate::core::state::AppState;

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    active_plugins: Vec<usize>,
    event_bus: SharedEventBus,
    hook_manager: HookManager,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            active_plugins: Vec::new(),
            event_bus: super::events::new_shared_event_bus(),
            hook_manager: HookManager::new(),
        }
    }

    pub fn event_bus(&self) -> SharedEventBus {
        self.event_bus.clone()
    }

    pub fn hook_manager(&self) -> &HookManager {
        &self.hook_manager
    }

    pub fn hook_manager_mut(&mut self) -> &mut HookManager {
        &mut self.hook_manager
    }

    /// Register a plugin
    pub fn register(
        &mut self,
        mut plugin: Box<dyn Plugin>,
        document: &mut Document,
        state: &mut AppState,
    ) {
        let info = plugin.info();
        log::info!(
            "Registering plugin: {} v{} by {}",
            info.name,
            info.version,
            info.author
        );

        let selected = state.selected_ids.clone();
        let mut ctx = PluginContextMut {
            document,
            state,
            selected_ids: &selected,
        };
        plugin.on_load(&mut ctx);

        let idx = self.plugins.len();
        self.plugins.push(plugin);
        self.active_plugins.push(idx);

        // Emit event
        if let Ok(bus) = self.event_bus.read() {
            bus.emit(&Event::Custom {
                source: "plugin_manager".into(),
                name: "plugin_registered".into(),
                payload: Some(info.id.into()),
            });
        }
    }

    /// Unregister a plugin by ID
    pub fn unregister(&mut self, plugin_id: &str, document: &mut Document, state: &mut AppState) {
        if let Some(idx) = self.plugins.iter().position(|p| p.info().id == plugin_id) {
            let selected = state.selected_ids.clone();
            let mut ctx = PluginContextMut {
                document,
                state,
                selected_ids: &selected,
            };
            self.plugins[idx].on_unload(&mut ctx);
            self.hook_manager.unregister(plugin_id);
            self.plugins.remove(idx);
            self.active_plugins.retain(|&i| i != idx);
            log::info!("Unregistered plugin: {}", plugin_id);
        }
    }

    /// Get info about all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|p| p.info()).collect()
    }

    /// Toggle plugin active state
    pub fn toggle_plugin(
        &mut self,
        plugin_id: &str,
        document: &mut Document,
        state: &mut AppState,
    ) {
        let selected = state.selected_ids.clone();
        let mut ctx = PluginContextMut {
            document,
            state,
            selected_ids: &selected,
        };

        if let Some(idx) = self.plugins.iter().position(|p| p.info().id == plugin_id) {
            if self.active_plugins.contains(&idx) {
                self.plugins[idx].on_deactivate(&mut ctx);
                self.active_plugins.retain(|&i| i != idx);
            } else {
                self.plugins[idx].on_activate(&mut ctx);
                self.active_plugins.push(idx);
            }
        }
    }

    /// Dispatch an action to the plugin that registered it
    pub fn on_action(
        &mut self,
        plugin_id: &str,
        action: &str,
        document: &mut Document,
        state: &mut AppState,
    ) {
        let selected = state.selected_ids.clone();
        let mut ctx = PluginContextMut {
            document,
            state,
            selected_ids: &selected,
        };

        if let Some(idx) = self.plugins.iter().position(|p| p.info().id == plugin_id) {
            self.plugins[idx].on_action(action, &mut ctx);
        }
    }

    /// Collect all menu entries from active plugins
    pub fn collect_menu_entries(&self) -> Vec<(String, super::api::PluginMenuEntry)> {
        let mut entries = Vec::new();
        for &idx in &self.active_plugins {
            let plugin = &self.plugins[idx];
            let info = plugin.info();
            for entry in plugin.menu_entries() {
                entries.push((info.name.to_string(), entry));
            }
        }
        entries
    }

    /// Collect all panel definitions from active plugins
    pub fn collect_panels(&self) -> Vec<super::api::PluginPanel> {
        let mut panels = Vec::new();
        for &idx in &self.active_plugins {
            panels.extend(self.plugins[idx].ui_panels());
        }
        panels
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
