use std::env;
use std::fs;
use std::io;
use std::path::Path;

use rupert_macros::{partial_derive, partial_struct, Partial};
use serde::{Deserialize, Serialize};
use toml;

use crate::presentation;

/// The environment variable used to find the configuration file.
const CONFIGURATION_FILE_PATH_ENV: &str = "RUPERT_CONFIGURATION_FILE";

/// The application configuration file.
#[derive(Deserialize, Serialize, Partial)]
#[partial_derive(Clone, Deserialize, Serialize)]
#[partial_struct(ConfigurationFragment)]
pub struct Configuration {
    /// The title of the presentation.
    #[partial_default("Presentation".into())]
    pub title: String,

    /// The page break configuration.
    #[partial_default(presentation::PageBreakCondition::ThematicBreak)]
    pub page_break: presentation::PageBreakCondition,
}

/// Loads the application configuration.
///
/// If the environment variable `RUPERT_CONFIGURATION_FILE` is set, the
/// configuration is loaded from that file, otherwise a default value is used.
pub fn load() -> io::Result<ConfigurationFragment> {
    Ok([env::var(CONFIGURATION_FILE_PATH_ENV).ok().map(load_from)]
        .into_iter()
        .filter_map(|i| i)
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .fold(ConfigurationFragment::default(), |acc, partial| {
            acc.merge(partial)
        }))
}

/// Loads a configuration from a TOML file.
///
/// # Arguments
/// *  `path` - The file to load.
fn load_from<P>(path: P) -> io::Result<ConfigurationFragment>
where
    P: AsRef<Path>,
{
    toml::from_str(&fs::read_to_string(&path)?)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
