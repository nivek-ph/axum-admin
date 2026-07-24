pub mod app;
pub mod cli;
pub(crate) mod commands;

pub use crate::commands::serve::ServeConfig;

pub fn install_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}
