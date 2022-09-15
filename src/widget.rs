use std::iter::repeat;

use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::presentation::Page;
use crate::transform::{Section, Sections};

/// A widget representing a page.
pub struct PageWidget<'a> {
    /// The sections of the page.
    sections: Sections<'a>,
}

impl<'a> Widget for &'a PageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.sections.render(area, buf);
    }
}

impl<'a> From<&'a Page<'a>> for PageWidget<'a> {
    fn from(source: &'a Page<'a>) -> Self {
        Self {
            sections: source.into(),
        }
    }
}

impl<'a> Sections<'a> {
    /// Calculates the required height for these sections given a width.
    ///
    /// # Arguments
    /// *  `width` - The width of the rendering area.
    pub fn height(&self, width: u16) -> u16 {
        self.iter()
            .enumerate()
            .map(|(i, section)| {
                self.height_of(section, width, i == 0, i == self.len() - 1)
            })
            .sum()
    }

    /// Calculates the required height for a single section.
    ///
    /// # Arguments
    /// *  `section` - The section shose height to calculate.
    /// *  `width` - The width of the rendering area.
    /// *  `is_first` - Whether this section is the first section.
    /// *  `is_last` - Whether this section is the last section.
    fn height_of(
        &self,
        section: &Section<'a>,
        width: u16,
        is_first: bool,
        is_last: bool,
    ) -> u16 {
        let padding = section.padding();
        section.height(width)
            + if is_first { 0 } else { padding.0 }
            + if is_last {
                0
            } else {
                padding.1 + self.inner_margin
            }
    }
}

impl<'a> Widget for &'a Sections<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                self.iter()
                    .enumerate()
                    .map(|(i, section)| {
                        Constraint::Length(self.height_of(
                            section,
                            area.width,
                            i == 0,
                            i == self.len() - 1,
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
            .split(area);
        for (i, (mut part, section)) in
            parts.into_iter().zip(self.iter()).enumerate()
        {
            let padding = section.padding();
            let is_first = i == 0;
            let is_last = i == self.len() - 1;
            if !is_first {
                part.y += padding.0;
                part.height -= padding.0;
            }
            if !is_last {
                part.height -= padding.1 + self.inner_margin;
            }
            section.render(part, buf);
        }
    }
}

impl<'a> Section<'a> {
    /// Calculates the required height for this section given a width.
    ///
    /// # Arguments
    /// *  `width` - The width of the rendering area.
    pub fn height(&self, width: u16) -> u16 {
        use Section::*;
        match self {
            BlockQuote { content } => Self::height_block_quote(width, content),
            Code { text } => Self::height_code(width, text),
            Heading { text, level } => Self::height_heading(width, text, level),
            List { content } => Self::height_list(width, content),
            ListItemOrdered {
                content,
                ordinal,
                delimiter,
            } => Self::height_list_item_ordered(
                width, content, ordinal, delimiter,
            ),
            ListItemUnordered { content, bullet } => {
                Self::height_list_item_unordered(width, content, bullet)
            }
            Paragraph { text } => Self::height_paragraph(width, text),
            ThematicBreak => Self::height_thematic_break(width),
        }
    }

    /// The top and bottom padding for this section.
    ///
    /// The padding is only used if the section has a neighbour in the
    /// respective direction.
    pub fn padding(&self) -> (u16, u16) {
        use Section::*;
        match self {
            Heading { .. } => (1, 0),
            _ => (0, 0),
        }
    }

    fn height_block_quote(width: u16, content: &Sections<'a>) -> u16 {
        // We add 2 for the head and tail lines
        2 + content.height(width)
    }

    fn height_code(_width: u16, text: &Text<'a>) -> u16 {
        // We do not wrap code sections, so the height is the number of lines
        text.height() as u16
    }

    fn height_heading(width: u16, text: &Spans<'a>, level: &u8) -> u16 {
        // A heading is a single line, with an additional header determined by
        // the level
        Self::height_line(width, *level as u16 + 1, &text.0)
    }

    fn height_list(width: u16, content: &Sections<'a>) -> u16 {
        // The height of a list is the height of its sections
        content.height(width)
    }

    fn height_list_item_ordered(
        width: u16,
        content: &Sections<'a>,
        _ordinal: &usize,
        _delimiter: &char,
    ) -> u16 {
        // The height of a list item is the height of its sections
        content.height(width)
    }

    fn height_list_item_unordered(
        width: u16,
        content: &Sections<'a>,
        _bullet: &char,
    ) -> u16 {
        // The height of a list item is the height of its sections
        content.height(width)
    }

    fn height_paragraph(width: u16, text: &Text<'a>) -> u16 {
        // The height of a paragraph is the height of its wrapped lines if it
        // contains any non-whitespace characters
        /*if text
            .lines
            .iter()
            .any(|line| Self::contains_non_whitespace(&line.0))
        {*/
        text.lines
            .iter()
            .map(|line| Self::height_line(width, 0, &line.0))
            .sum::<u16>()
        /*} else {
            0
        }*/
    }

    fn height_thematic_break(_width: u16) -> u16 {
        // A thematic break is always one lines high
        1
    }

    /// Calculates the height of a single line.
    ///
    /// This function takes wrapping of long lines into account.
    ///
    /// # Arguments
    /// *  `width` - The width of the rendering area.
    /// *  `indent` - An initial assumed indent.
    /// *  `value` - The line for which to calculate the height.
    fn height_line(width: u16, indent: u16, value: &[Span<'_>]) -> u16 {
        enum Word {
            None,
            Started(u16),
            WrappedAt(u16),
        }
        struct State {
            height: u16,
            pos: u16,
            current: Word,
        }
        value
            .iter()
            .flat_map(|span| span.content.chars())
            .enumerate()
            .fold(
                State {
                    height: 1,
                    pos: indent,
                    current: Word::None,
                },
                |mut state, (i, c)| {
                    use Word::*;

                    state.pos += 1;
                    state.current = match state.current {
                        // Start a new word if none active when we
                        // encounter non-whitesspace
                        None if !c.is_whitespace() => Started(i as u16),

                        // Stop current word on whitespace
                        Started(_) if c.is_whitespace() => None,

                        // Wrap when we encounter end of line
                        Started(pos) if state.pos >= width => {
                            WrappedAt(i as u16 - pos)
                        }

                        // Add wrapped word to next line at the end of the
                        // word, unless the next line is empty
                        WrappedAt(pos) if c.is_whitespace() => {
                            state.pos = if state.pos > 0 {
                                state.pos + pos
                            } else {
                                state.pos
                            };
                            None
                        }
                        _ => state.current,
                    };

                    // Increase height and start from the beginning when we
                    // encounter end of line
                    if state.pos >= width {
                        state.pos = 0;
                        state.height += 1;
                    }
                    state
                },
            )
            .height
    }

    /// Determines whether a collection of spans contains non-whitespace.
    ///
    /// # Argument
    /// *  `line` - The line to check.
    fn contains_non_whitespace(line: &[Span<'a>]) -> bool {
        line.iter()
            .flat_map(|span| span.content.chars())
            .any(|c| !c.is_whitespace())
    }
}

impl<'a> Section<'a> {
    /// Renders this section.
    ///
    /// # Arguments
    /// *  `area` - The allocated area for this section.
    /// *  `buf` - The target buffer.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        use Section::*;
        match &self {
            BlockQuote { content } => {
                Self::render_block_quote(area, buf, &content)
            }
            Code { text } => Self::render_code(area, buf, text),
            Heading { text, level } => {
                Self::render_heading(area, buf, text, level)
            }
            List { content } => Self::render_list(area, buf, &content),
            ListItemOrdered {
                content,
                ordinal,
                delimiter,
            } => Self::render_list_item_ordered(
                area, buf, &content, ordinal, delimiter,
            ),
            ListItemUnordered { content, bullet } => {
                Self::render_list_item_unordered(area, buf, &content, bullet)
            }
            Paragraph { text } => Self::render_paragraph(area, buf, text),
            ThematicBreak => Self::render_thematic_break(area, buf),
        }
    }

    fn render_block_quote(
        area: Rect,
        buf: &mut Buffer,
        content: &Sections<'a>,
    ) {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Max(area.height),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(area);
        Paragraph::new(">>>").render(parts[0], buf);
        Paragraph::new("<<<").render(parts[2], buf);
        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(Self::INDENT / 2),
                    Constraint::Max(area.width),
                ]
                .as_ref(),
            )
            .split(parts[1]);
        content.render(parts[1], buf);
    }

    fn render_code(area: Rect, buf: &mut Buffer, text: &Text<'a>) {
        Paragraph::new(text.clone()).render(area, buf);
    }

    fn render_heading(
        area: Rect,
        buf: &mut Buffer,
        text: &Spans<'a>,
        level: &u8,
    ) {
        Paragraph::new({
            let mut text = text.clone();
            text.0.insert(
                0,
                Span::raw(
                    repeat('#').take(*level as usize).collect::<String>() + " ",
                ),
            );
            text
        })
        .wrap(Wrap { trim: true })
        .render(area, buf);
    }

    fn render_list(area: Rect, buf: &mut Buffer, content: &Sections<'a>) {
        content.render(area, buf);
    }

    fn render_list_item_ordered(
        area: Rect,
        buf: &mut Buffer,
        content: &Sections<'a>,
        ordinal: &usize,
        delimiter: &char,
    ) {
        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(Self::INDENT),
                    Constraint::Max(area.width),
                ]
                .as_ref(),
            )
            .split(area);
        Paragraph::new(format!("{}{}", ordinal, delimiter))
            .render(parts[0], buf);
        content.render(parts[1], buf);
    }

    fn render_list_item_unordered(
        area: Rect,
        buf: &mut Buffer,
        content: &Sections<'a>,
        bullet: &char,
    ) {
        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(Self::INDENT),
                    Constraint::Max(area.width),
                ]
                .as_ref(),
            )
            .split(area);
        Paragraph::new(format!("{}", bullet)).render(parts[0], buf);
        content.render(parts[1], buf);
    }

    fn render_paragraph(area: Rect, buf: &mut Buffer, text: &Text<'a>) {
        if text
            .lines
            .iter()
            .any(|line| Self::contains_non_whitespace(&line.0))
        {
            Paragraph::new(text.clone())
                .wrap(Wrap { trim: true })
                .render(area, buf);
        }
    }

    fn render_thematic_break(area: Rect, buf: &mut Buffer) {
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::White))
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_line() {
        assert_eq!(1, Section::height_line(10, 0, &["one".into()]));
        assert_eq!(2, Section::height_line(10, 0, &["one two three".into()]));
        assert_eq!(
            2,
            Section::height_line(10, 0, &["one two".into(), " three".into()]),
        );
        assert_eq!(
            5,
            Section::height_line(
                10,
                0,
                &["a long wooooooooooooooooooooooooooooooooooord".into()]
            )
        );
    }
}
