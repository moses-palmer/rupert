use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml;

use crate::presentation;

/// The application configuration file.
#[derive(Deserialize, Serialize)]
pub struct Configuration {
    /// The title of the presentation.
    pub title: String,

    /// Information about the source.
    pub source: Source,

    /// The page break configuration.
    pub page_break: Option<presentation::PageBreakCondition>,
}

/// Information about the source.
#[derive(Deserialize, Serialize)]
pub struct Source {
    /// The path to the source document.
    pub path: String,
}

/// Loads a configuration from a TOML file.
///
/// # Arguments
/// *  `path` - The path to the configuration file.
///
/// # Panics
/// This function will panic if no parent directory for `path` can be found,
/// and the current directory cannot be determined.
pub fn load<P>(path: P) -> io::Result<(PathBuf, Configuration)>
where
    P: AsRef<Path>,
{
    toml::from_str(&fs::read_to_string(&path)?)
        .map(|configuration| {
            (
                path.as_ref()
                    .parent()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| env::current_dir().unwrap()),
                configuration,
            )
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
