use std::env;
use std::path;
use std::process;

mod configuration;
mod presentation;

fn run<P>(
    path: P,
    _configuration: configuration::Configuration,
) -> Result<(), String>
where
    P: AsRef<path::Path>,
{
    let arena = comrak::Arena::new();
    let _presentation = presentation::load(&arena, &path).map_err(|e| {
        format!(
            "Failed to load markdown document {}: {}",
            path.as_ref().to_string_lossy(),
            e
        )
    })?;

    Ok(())
}

/// Initialises the application and returns the root directory and
/// configuration.
///
/// # Panics
/// This function will panic if the current executable name cannot be
/// determined.
fn initialize() -> Result<(path::PathBuf, configuration::Configuration), String>
{
    let presentation = env::args().skip(1).next().ok_or_else(usage)?;
    let configuration = configuration::load()
        .map_err(|e| format!("Failed to load configuration: {}", e))?;
    Ok((presentation.into(), configuration))
}

/// The usage string.
///
/// # Panics
/// This function will panic if the current executable name cannot b dtermined.
fn usage() -> String {
    let name = env::current_exe()
        .map(|exe| exe.to_string_lossy().into_owned())
        .unwrap();
    format!("Usage: {} PRESENTATION", name)
}

fn main() {
    match initialize().and_then(|(presentation, configuration)| {
        run(presentation, configuration)
    }) {
        Ok(_) => process::exit(0),
        Err(s) => {
            eprintln!("{}", s);
            process::exit(1);
        }
    }
}
