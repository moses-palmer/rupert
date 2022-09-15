use std::io;

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use tui::backend::CrosstermBackend;

use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::widgets::{Block, BorderType, Borders, Gauge};
use tui::Frame;

use crate::configuration::Configuration;
use crate::widget::PageWidget;

/// Runs the UI main loop.
///
/// This function will not return until the user exits.
///
/// # Arguments
/// *  `configuraiton` - The application configuration.
/// *  `pages` - The pages of the presentation.
pub fn run(
    configuration: &Configuration,
    pages: Vec<PageWidget>,
) -> Result<(), String> {
    let mut terminal = Terminal::new()?;
    let mut page = 0usize;

    #[allow(unused_must_use)]
    loop {
        terminal
            .0
            .draw(|frame| render(frame, configuration, &pages, page))
            .map(|_| ())
            .or_else(|_| terminal.0.clear())
            .map_err(|e| format!("Failed to render TUI: {}", e));
        if let Event::Key(key) =
            event::read().map_err(|e| format!("Failed to read event: {}", e))?
        {
            match key.code {
                KeyCode::Left | KeyCode::Backspace => {
                    if page > 0 {
                        page -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Enter => {
                    if page < pages.len() - 1 {
                        page += 1;
                    }
                }
                KeyCode::Char('q') => break,
                _ => continue,
            }
        }
    }

    Ok(())
}

fn render(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    configuration: &Configuration,
    widgets: &Vec<PageWidget<'_>>,
    page: usize,
) {
    let size = frame.size();

    let show_progress = widgets.len() > 1;
    let progress_height = if show_progress { 1 } else { 0 };

    // The layout for the presentation and the progress gauge
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(size.height - progress_height),
                Constraint::Length(progress_height),
            ]
            .as_ref(),
        )
        .split(size);

    // The window containing the presentation and the rectangle for content
    let presentation_window = Block::default()
        .borders(Borders::ALL)
        .title(configuration.title.as_str())
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    let content_rect = presentation_window.inner(main_layout[0]);

    frame.render_widget(presentation_window, main_layout[0]);
    frame.render_widget(&widgets[page], content_rect);

    if show_progress {
        let progress = if widgets.len() > 1 {
            page as f64 / (widgets.len() - 1) as f64
        } else {
            0.0
        };
        let progress = Gauge::default()
            .ratio(progress)
            .label("")
            .use_unicode(true)
            .gauge_style(Style::default().fg(Color::Gray).bg(Color::DarkGray));
        frame.render_widget(progress, main_layout[1]);
    }
}

struct Terminal(pub tui::Terminal<CrosstermBackend<io::Stdout>>);

impl Terminal {
    pub fn new() -> Result<Self, String> {
        crossterm::terminal::enable_raw_mode()
            .map_err(|e| format!("Failed to initialise terminal: {}", e))?;

        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)
            .map_err(|e| format!("Failed to initialise terminal: {}", e))?;

        let backend = CrosstermBackend::new(stdout);

        tui::Terminal::new(backend)
            .map_err(|e| format!("Failed to initialise terminal: {}", e))
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
