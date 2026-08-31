pub mod canvas;
pub mod panels;
pub mod timeline_widget;

pub use canvas::CanvasWidget;
pub use panels::{
    AlignPanel, EffectsPanel, FormulaPanel, LayerPanel, MorphPanel, OffsetPanel, PathfinderPanel,
    PresetPanel, PropertyPanel, TracePanel, VfxTrailPanel,
};
pub use timeline_widget::TimelineWidget;
