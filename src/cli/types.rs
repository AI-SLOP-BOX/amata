use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliBooleanOp {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliCurveType {
    Spiral,
    Lissajous,
    Spirograph,
    Rose,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliHalftonePattern {
    Circular,
    Hex,
    Scanline,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliIsoPlane {
    Top,
    Left,
    Right,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliLSystemPreset {
    Tree,
    Dragon,
    Snowflake,
    Hilbert,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliDeformType {
    Wave,
    Noise,
    Glitch,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliFlowFieldPreset {
    Vortex,
    Magnetic,
    Cyber,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliBlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliWaveformType {
    Sine,
    Sawtooth,
    Square,
    Triangle,
    Harmonics,
    Fm,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliWarpPreset {
    Bulge,
    Pinch,
    Twist,
    Wave,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliGradientMeshPreset {
    Sunset,
    Cyberpunk,
    Aurora,
    Gold,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliAxonometricMode {
    Isometric,
    Dimetric,
    Trimetric,
    Cabinet,
    Cavalier,
}
