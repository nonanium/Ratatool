mod capabilities;
mod detect;

pub use capabilities::{
    CapabilityPolicy, Charset, ColorDepth, ModifierSupport, RenderClass, TerminalOptions,
    TerminalOverrides, TerminalProfile,
};
pub use detect::{DetectionInput, detect_current, detect_profile};
