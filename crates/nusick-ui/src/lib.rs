//! # nusick-ui
//!
//! The shared look of Nusick plugins: a dark egui theme plus a few
//! parameter-aware widgets. It is kept small on purpose.
//!
//! - [`theme::apply`] sets up colors and spacing on an egui context.
//! - [`Knob`] is a rotary control bound to a nih-plug parameter. Drag
//!   vertically to change it, Shift-drag for fine control, double-click to reset.
//! - [`Meter`] is a vertical peak meter in dBFS.

pub mod knob;
pub mod meter;
pub mod theme;

pub use knob::Knob;
pub use meter::Meter;
pub use nih_plug_egui::egui;
