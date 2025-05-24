#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod background;
pub mod clipboard;
mod egui_utils;
mod host;
pub use app::FirefoxSessionDataApp;
