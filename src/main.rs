use std::env;
use std::path;
use std::process;

mod configuration;
mod presentation;
mod transform;
mod widget;

mod ui;

fn run<P>(
    path: P,
    configuration: configuration::ConfigurationFragment,
) -> Result<(), String>
where
    P: AsRef<path::Path>,
{
    let arena = comrak::Arena::new();
    let presentation = presentation::load(&arena, &path).map_err(|e| {
        format!(
            "Failed to load markdown document {}: {}",
            path.as_ref().to_string_lossy(),
            e
        )
    })?;

    let configuration = configuration::Configuration::from(
        presentation
            .configuration()
            .map(|c| {
                Ok::<_, String>(configuration.clone().merge(c.map_err(
                    |e| {
                        format!(
                        "Failed to read configuration from presentation: {}",
                        e,
                    )
                    },
                )?))
            })
            .unwrap_or_else(|| Ok(configuration))?,
    );

    let pages = Ok(presentation
        .pages(configuration.page_break.clone())
        .collect::<Vec<_>>())
    .and_then(|pages| {
        if pages.len() < 1 {
            Err(format!("Invalid presentation: no pages"))
        } else {
            Ok(pages)
        }
    })?;

    let page_collector = widget::PageCollector::collect(&configuration, &pages);
    let (context, widgets) = page_collector.finish();

    ui::run(path, &configuration, &context, widgets)
}

/// Initialises the application and returns the root directory and
/// configuration.
///
/// # Panics
/// This function will panic if the current executable name cannot be
/// determined.
fn initialize(
) -> Result<(path::PathBuf, configuration::ConfigurationFragment), String> {
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
