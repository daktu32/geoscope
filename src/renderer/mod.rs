// renderer/mod.rs — Hub for all renderer modules

pub mod common;
pub mod export;
pub mod globe;
pub mod hovmoller;
pub mod map;
pub mod spectrum;

pub use globe::GlobeRenderer;
pub use map::MapRenderer;
