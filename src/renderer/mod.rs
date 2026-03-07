// renderer/mod.rs — Hub for all renderer modules

pub mod common;
pub mod cross_section;
pub mod export;
pub mod globe;
pub mod hovmoller;
pub mod map;
pub mod spectrum;
pub mod vector_overlay;

pub use globe::GlobeRenderer;
pub use map::MapRenderer;
