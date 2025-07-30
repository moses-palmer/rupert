use std::io;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::configuration::Configuration;
use crate::transform::{Context, color};
use crate::widget::PageWidget;

/// Runs the UI main loop.
///
/// This function will not return until the user exits.
///
/// # Arguments
/// *  `path` - The path to the presentation to display.
/// *  `configuration` - The application configuration.
/// *  `context` - A presentation context.
/// *  `pages` - The pages of the presentation.
pub fn run<P>(
    path: P,
    configuration: &Configuration,
    context: &Context,
    pages: Vec<PageWidget>,
) -> Result<(), String>
where
    P: AsRef<Path>,
{
    let mut terminal = Terminal::new()?;
    let mut page = 0usize;

    configuration.commands.initialize(&path);

    #[allow(unused_must_use)]
    loop {
        terminal
            .0
            .draw(|frame| render(frame, configuration, context, &pages, page))
            .map(|_| ())
            .or_else(|_| terminal.0.clear())
            .map_err(|e| format!("Failed to render TUI: {e}"));
        if let Event::Key(key) =
            event::read().map_err(|e| format!("Failed to read event: {e}"))?
        {
            match key.code {
                KeyCode::Left | KeyCode::Backspace => {
                    page = page.saturating_sub(1);
                }
                KeyCode::Right | KeyCode::Enter => {
                    if page < pages.len() - 1 {
                        page += 1;
                    }
                }
                KeyCode::Char('q') => break,
                _ => continue,
            }

            configuration.commands.update(&path, page + 1, pages.len());
        }
    }

    configuration.commands.finalize(&path);

    Ok(())
}

fn render(
    frame: &mut Frame,
    configuration: &Configuration,
    context: &Context,
    widgets: &[PageWidget<'_>],
    page: usize,
) {
    let area = frame.area();

    // The window containing the presentation and the rectangle for content
    let presentation_window = Block::default()
        .style(
            Style::default().bg(context
                .theme
                .settings
                .background
                .as_ref()
                .map(color)
                .unwrap_or_else(|| Color::Black)),
        )
        .borders(Borders::ALL)
        .title(configuration.title.as_str())
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    let content_rect = presentation_window.inner(area);

    // The layout for the presentation and the page display
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [Constraint::Length(area.height - 3), Constraint::Length(1)]
                .as_ref(),
        )
        .split(content_rect);

    frame.render_widget(presentation_window, area);
    frame.render_widget(&widgets[page], main_layout[0]);
    frame.render_widget(
        Paragraph::new(Text::raw(format!("{} / {}", page + 1, widgets.len())))
            .alignment(Alignment::Right),
        main_layout[1],
    );
}

struct Terminal(pub ratatui::Terminal<CrosstermBackend<io::Stdout>>);

impl Terminal {
    pub fn new() -> Result<Self, String> {
        crossterm::terminal::enable_raw_mode()
            .map_err(|e| format!("Failed to initialise terminal: {e}"))?;

        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)
            .map_err(|e| format!("Failed to initialise terminal: {e}"))?;

        let backend = CrosstermBackend::new(stdout);

        ratatui::Terminal::new(backend)
            .map_err(|e| format!("Failed to initialise terminal: {e}"))
            .map(Self)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        crossterm::terminal::disable_raw_mode().unwrap();
        execute!(self.0.backend_mut(), LeaveAlternateScreen).unwrap();
        self.0.show_cursor().unwrap();
    }
}
