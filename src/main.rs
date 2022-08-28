use std::env;
use std::path;
use std::process;

mod configuration;
mod presentation;

fn run<P>(
    root: P,
    configuration: configuration::Configuration,
) -> Result<(), String>
where
    P: AsRef<path::Path>,
{
    let arena = comrak::Arena::new();
    let presentation = presentation::load(
        &arena,
        root.as_ref().join(&configuration.source.path),
    )
    .map_err(|e| {
        format!(
            "Failed to load markdown document {}: {}",
            configuration.source.path, e
        )
    })?;

    let _pages = Ok(presentation
        .pages(configuration.page_break.clone().unwrap_or_default())
        .collect::<Vec<_>>())
    .and_then(|pages| {
        if pages.len() < 1 {
            Err(format!("Invalid presentation: no pages"))
        } else {
            Ok(pages)
        }
    });

    Ok(())
}

/// Initialises the application and returns the root directory and
/// configuration.
///
/// # Panics
/// This function will panic if the current executable name cannot b dtermined.
fn initialize() -> Result<(path::PathBuf, configuration::Configuration), String>
{
    let name = env::current_exe()
        .map(|exe| exe.to_string_lossy().into_owned())
        .unwrap();
    let configuration_file = env::args()
        .skip(1)
        .next()
        .ok_or_else(|| format!("Usage: {} CONFIGURATION_FILE", name))?;
    configuration::load(&configuration_file)
        .map_err(|e| format!("Failed to load {}: {}", configuration_file, e))
}

fn main() {
    match initialize()
        .and_then(|(root, configuration)| run(root, configuration))
    {
        Ok(_) => process::exit(0),
        Err(s) => {
            eprintln!("{}", s);
            process::exit(1);
        }
    }
}
