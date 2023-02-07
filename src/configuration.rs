use std::env;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::Path;
use std::process;

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

    /// The various commands executed during presentation.
    pub commands: Commands,
}

/// The various commands executed during presentation.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Commands {
    /// The command executed after the presentation has been loaded.
    pub initialize: Option<Command>,
}

impl Commands {
    /// Calls the `initialize` command.
    ///
    /// # Arguments
    /// *  `path` - The path to the presentation.
    pub fn initialize<P>(&self, path: P)
    where
        P: AsRef<Path>,
    {
        self.dispatch(&path, &self.initialize, |_| None)
    }

    /// Dispatches execution to an optional command.
    ///
    /// The result of the execution is discarded, but written to `stderr`.
    ///
    /// # Arguments
    /// *  `path` - The path to the presentation.
    /// *  `command` - The optional command to execute.
    /// *  `replacements` - A function converting keys to replacement strings.
    ///    The key `"presentation.path"` will always be set to the absolute
    ///    path of the presentation.
    fn dispatch<'a, F, P>(
        &self,
        path: P,
        command: &Option<Command>,
        replacements: F,
    ) where
        F: Fn(&str) -> Option<&'a str> + 'a,
        P: AsRef<Path>,
    {
        let cwd = match path
            .as_ref()
            .parent()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| ".".into())
            .canonicalize()
        {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Failed to retrieve path to presentation: {}", e);
                return;
            }
        };
        let absolute = match path.as_ref().canonicalize() {
            Ok(path) => path,
            Err(e) => {
                eprintln!(
                    "Failed to generate canonical path to presentation: {}",
                    e,
                );
                return;
            }
        };
        let presentation_path = absolute.to_string_lossy();
        match command.as_ref().map(|command| {
            command.execute(cwd, |key| match key {
                "presentation.path" => Some(&presentation_path),
                k => replacements(k),
            })
        }) {
            Some(Ok(exit_code)) => {
                if !exit_code.success() {
                    eprintln!(
                        "Command {:?} failed: {}",
                        command.as_ref().unwrap(),
                        exit_code
                    );
                }
            }
            Some(Err(error)) => {
                eprintln!(
                    "Failed to execute {:?}: {}",
                    command.as_ref().unwrap(),
                    error
                );
            }
            _ => {}
        }
    }
}

/// A description of a command to execute.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Command {
    /// The name of the binary.
    pub binary: String,

    /// Arguments to pass to the binary.
    pub arguments: Vec<String>,
}

impl Command {
    /// Executes this command with arguments interpolated.
    ///
    /// Parts of the string matching the format `"${token.name}"` will be
    /// converted as `replacements("token.name")`, and the string is replaced
    /// if a value is returned.
    ///
    /// # Arguments
    /// *  `cwd` - The current working directory for the command.
    /// *  `replacements` - A function converting keys to replacement strings.
    pub fn execute<'a, F, P>(
        &self,
        cwd: P,
        replacements: F,
    ) -> Result<process::ExitStatus, io::Error>
    where
        F: Fn(&str) -> Option<&'a str> + 'a,
        P: AsRef<Path>,
    {
        process::Command::new(&self.binary)
            .args(self.arguments.iter().map(|argument| {
                interpolate(&argument, |key| replacements(key))
            }))
            .current_dir(cwd)
            .spawn()?
            .wait()
    }
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

/// Interpolates all replacements in `string` given replacements in
/// `replacements`.
///
/// Tokens for which `replacements` returns `None` are kept.
///
/// # Arguments
/// *  `string` - The string to interpolate.
/// *  `replacements` - A function converting keys to replacement strings.
fn interpolate<'a, F>(string: &str, replacements: F) -> String
where
    F: Fn(&str) -> Option<&'a str> + 'a,
{
    let mut text = string.to_string();
    let mut index = 0;
    while let Some((replacement_range, key_range)) =
        next_replacement(index, &text)
    {
        let key = &text[key_range.clone()];
        if let Some(replacement) = replacements(key).map(str::to_string) {
            index += replacement_range.start + replacement.len();
            text = text.clone();
            text.replace_range(replacement_range, &replacement);
        } else {
            index += replacement_range.start + key.len();
        }
    }
    text
}

/// Finds the range to be replaced by the next replacement token, and the
/// range of the token itself.
///
/// Since a replacement token is marked with `"${token}"`, the replacement
/// token will always be a subset of the text to be replcaed.
///
/// # Arguments
/// *  `offset` - The start offset. Characters before this will be ignored.
/// *  `string` - The string in which to search.
fn next_replacement(
    offset: usize,
    string: &str,
) -> Option<(Range<usize>, Range<usize>)> {
    enum State {
        BeforeStart,
        Start(usize),
        Key(usize, usize),
    }
    let mut state = State::BeforeStart;

    use State::*;
    for (i, c) in string.chars().enumerate().skip(offset) {
        state = match (state, c) {
            (BeforeStart, '$') => Start(i),
            (Start(p), '{') => Key(p, i + 1),
            (Key(p, k), '}') => return Some((p..i + 1, k..i)),
            (Key(p, k), _) => Key(p, k),
            _ => BeforeStart,
        };
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_simple() {
        assert_eq!(
            "replacement 1, replacement 2, ${r3}",
            interpolate("${r1}, ${r2}, ${r3}".into(), |r| match r {
                "r1" => Some(&"replacement 1"),
                "r2" => Some(&"replacement 2"),
                _ => None,
            }),
        );
    }
}
