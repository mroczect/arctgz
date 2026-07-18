mod core;
pub mod handler;

pub use core::{init, load_config, save_config};
pub use handler::ArctgzConfig;
pub use handler::ArctgzError;
