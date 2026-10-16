//! `netproxyd`: owns the engine lifecycle, the store and the authenticated control API.
pub mod bodies;
pub mod control;
pub mod control_client;
pub mod daemon;
pub mod flow_converter;
pub mod pattern_detector;
pub mod setup;
pub mod watcher;
