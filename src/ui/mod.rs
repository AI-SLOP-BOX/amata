pub mod canvas;
pub mod panels;
pub mod timeline_widget;

pub use canvas::CanvasWidget;
pub use panels::{
    AlignPanel, EffectsPanel, LayerPanel, MorphPanel, OffsetPanel, PathfinderPanel, PresetPanel,
    PropertyPanel, TracePanel,
};
pub use timeline_widget::TimelineWidget;
