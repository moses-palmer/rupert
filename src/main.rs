use std::process;

mod configuration;

fn run(_configuration: configuration::Configuration) -> Result<(), String> {
    unimplemented!();
}

/// Initialises the application and returns the root directory and
/// configuration.
///
/// # Panics
/// This function will panic if the current executable name cannot be
/// determined.
fn initialize() -> Result<configuration::Configuration, String> {
    configuration::load()
        .map_err(|e| format!("Failed to load configuration: {}", e))
}

fn main() {
    match initialize().and_then(|configuration| run(configuration)) {
        Ok(_) => process::exit(0),
        Err(s) => {
            eprintln!("{}", s);
            process::exit(1);
        }
    }
}
