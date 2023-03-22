use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Deref;

use comrak::arena_tree::Node;
use comrak::nodes::{Ast, ListDelimType, ListType, NodeValue};

use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color as SyntectColor, FontStyle, Theme, ThemeSet,
};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};

use crate::configuration::Configuration;
use crate::presentation::Page;

/// A collection of sections.
#[derive(Clone, Debug)]
pub struct Sections<'a> {
    /// The sections.
    sections: Vec<Section<'a>>,

    /// The margin between sections.
    pub inner_margin: u16,
}

impl<'a> Sections<'a> {
    /// Constructs sections from a page.
    ///
    /// # Arguments
    /// *  `context` - The context used during transform.
    /// *  `page` - The source page.
    pub fn from_page(context: &mut Context<'a>, page: &'a Page<'a>) -> Self {
        let mut sections = Vec::new();
        for source in page.nodes() {
            section(
                context,
                source,
                &mut sections,
                (&context.configuration.default_style).into(),
            );
        }
        sections.into()
    }

    /// Reorders all ordered list items in a list of sections.
    ///
    /// # Arguments
    /// *  `start_at` - The starting index.
    fn list_item_reorder(&mut self, start_at: usize) {
        self.sections
            .iter_mut()
            .filter(|section| match &section {
                Section::ListItemOrdered { .. } => true,
                _ => false,
            })
            .enumerate()
            .for_each(|(i, mut section)| match &mut section {
                Section::ListItemOrdered {
                    ref mut ordinal, ..
                } => *ordinal = start_at + i,
                _ => {}
            });
    }
}

impl<'a> Deref for Sections<'a> {
    type Target = [Section<'a>];

    fn deref(&self) -> &Self::Target {
        &self.sections
    }
}

impl<'a> From<Vec<Section<'a>>> for Sections<'a> {
    fn from(source: Vec<Section<'a>>) -> Self {
        {
            Self {
                sections: source,
                inner_margin: 1,
            }
        }
    }
}

/// A page section.
#[derive(Clone, Debug)]
pub enum Section<'a> {
    /// A block quote.
    BlockQuote {
        /// The content of the quote.
        content: Sections<'a>,
    },

    /// A code block.
    Code {
        /// The text of the section.
        text: Text<'a>,
    },

    /// A heading section.
    Heading {
        /// The text of the section.
        text: Spans<'a>,

        /// The heading level.
        level: u8,
    },

    /// A collection of list items.
    List {
        /// The content of the item.
        content: Sections<'a>,
    },

    /// A list item in an ordered list.
    ListItemOrdered {
        /// The content of the item.
        content: Sections<'a>,

        /// The ordinal of this item.
        ordinal: usize,

        /// The delimiter.
        delimiter: char,
    },

    /// A list item in an unordered list.
    ListItemUnordered {
        /// The content of the item.
        content: Sections<'a>,

        /// The bullet marker.
        bullet: char,
    },

    /// A paragraph.
    Paragraph {
        /// The text of the section.
        text: Text<'a>,
    },

    /// A table.
    Table {
        /// The table cells, as the cells of a row wrapped in a list of rows.
        rows: Vec<TableRow<'a>>,
    },

    /// A thematic break
    ThematicBreak,
}

impl<'a> Section<'a> {
    /// The number of cells each level of indentaion provides.
    pub const INDENT: u16 = 4;
}

/// The context used during transform.
#[derive(Clone)]
pub struct Context<'a> {
    /// The configuration for the presentation.
    pub configuration: &'a Configuration,

    /// The footnotes on the current page.
    pub footnotes: Footnotes<'a>,

    /// The known language syntaxes.
    pub syntax_set: SyntaxSet,

    /// The known language syntax highlighting themes.
    pub theme: Theme,
}

impl<'a> From<&'a Configuration> for Context<'a> {
    /// Constructs an empty context.
    fn from(source: &'a Configuration) -> Self {
        Self {
            configuration: source,
            footnotes: Default::default(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: ThemeSet::load_defaults()
                .themes
                .remove("base16-ocean.dark")
                .expect("failed to load theme"),
        }
    }
}

/// A list of footnotes.
#[derive(Clone, Debug)]
pub struct Footnotes<'a> {
    /// A set of references for the current page.
    references: HashSet<String>,

    /// The actual data.
    data: Vec<(String, Option<Sections<'a>>)>,
}

impl<'a> Footnotes<'a> {
    /// The characters used for numeric superscript.
    const SUPERSCRIPTS: [char; 10] =
        ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

    /// Adds a footnote reference.
    ///
    /// The return value is its index.
    ///
    /// # Arguments
    /// *  `name` - The footnote name.
    pub fn reference(&mut self, name: &str) -> usize {
        self.references.insert(name.into());
        if let Some(index) = self.data.iter().position(|(n, _)| name == n) {
            index
        } else {
            self.data.push((name.into(), None));
            self.data.len() - 1
        }
    }

    /// Declares a footnote
    ///
    /// The return value is its index.
    ///
    /// # Arguments
    /// *  `name` - The footnote name.
    /// *  `sections` - The footnote declaration.
    pub fn definition(&mut self, name: &str, sections: Sections<'a>) -> usize {
        if let Some(index) = self.data.iter().position(|(n, _)| name == n) {
            self.data[index].1 = Some(sections);
            index
        } else {
            self.data.push((name.into(), Some(sections)));
            self.data.len() - 1
        }
    }

    /// Locates a footnote by index and returns its declaration.
    ///
    /// # Arguments
    /// *  `index` - The footnote index.
    pub fn lookup(&self, index: usize) -> Option<&Sections<'a>> {
        self.data
            .get(index)
            .and_then(|(_, section)| section.as_ref())
    }

    /// Extracts the currently seen references and clears the list.
    pub fn extract_references(&mut self) -> Vec<usize> {
        let mut indices = self
            .references
            .iter()
            .filter_map(|name| self.data.iter().position(|(n, _)| name == n))
            .collect::<Vec<_>>();
        indices.sort();

        self.references.clear();
        indices
    }

    /// Converts an index to a superscript string.
    ///
    /// # Arguments
    /// *  `index` - The index to convert.
    pub fn index_to_superscript(index: usize) -> String {
        let mut current = index + 1;
        let mut result = Vec::new();
        while current > 0 {
            let i = current % 10;
            current /= 10;
            result.insert(0, Self::SUPERSCRIPTS[i]);
        }
        result.iter().collect()
    }
}

impl<'a> Default for Footnotes<'a> {
    fn default() -> Self {
        Self {
            references: HashSet::new(),
            data: Vec::new(),
        }
    }
}

/// A row in a table.
#[derive(Clone, Debug)]
pub struct TableRow<'a> {
    /// Whether this row is the header row.
    header: bool,

    /// The cells.
    cells: Vec<Text<'a>>,
}

impl<'a> TableRow<'a> {
    /// Creates a new table row.
    ///
    /// # Arguments
    /// *  `header` - Whether to create a header row.
    pub fn new(header: bool) -> Self {
        Self {
            header,
            cells: Vec::new(),
        }
    }

    /// Whether this row is a header row.
    pub fn header(&self) -> bool {
        self.header
    }

    /// The cells of this row.
    pub fn cells(&self) -> &[Text<'a>] {
        &self.cells
    }

    /// Adds a new cell to this row.
    ///
    /// # Arguments
    /// *  `cell` - The cell to add.
    pub fn push(&mut self, cell: Text<'a>) {
        self.cells.push(cell)
    }
}

/// Converts a _syntect_ colour to a _tui_ colour.
///
/// # Arguments
/// *  `color` - The colour to convert.
pub fn color(color: &SyntectColor) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Converts a collection of markdown AST nodes to sections.
///
/// # Arguments
/// *  `context` - The context used during transform.
/// *  `nodes` - The nodes to style.
/// *  `style` - The current style.
fn sections<'a>(
    context: &mut Context<'a>,
    source: &'a Node<'a, RefCell<Ast>>,
    target: &mut Vec<Section<'a>>,
    style: Style,
) {
    for source in source.children() {
        section(context, source, target, style);
    }
}

/// Handles a single block element.
///
/// # Arguments
/// *  `context` - The context used during transform.
/// *  `source` - The element to handle.
/// *  `target` - A target `Vec` for generated spans.
/// *  `style` - The current style.
fn section<'a>(
    context: &mut Context<'a>,
    source: &'a Node<'a, RefCell<Ast>>,
    target: &mut Vec<Section<'a>>,
    style: Style,
) {
    let node = &source.data.borrow().value;
    match node {
        NodeValue::BlockQuote => {
            let mut content = Vec::new();
            sections(
                context,
                source,
                &mut content,
                style
                    .add_modifier(Modifier::DIM)
                    .add_modifier(Modifier::ITALIC),
            );
            let content = content.into();
            target.push(Section::BlockQuote { content });
        }

        NodeValue::CodeBlock(code) => {
            let syntax = context
                .syntax_set
                .find_syntax_by_token(&String::from_utf8_lossy(&code.info))
                .unwrap_or_else(|| context.syntax_set.find_syntax_plain_text());
            let text = String::from_utf8_lossy(&code.literal).into_owned();
            let mut h = HighlightLines::new(syntax, &context.theme);
            let lines = LinesWithEndings::from(&text)
                .map(|line| {
                    h.highlight_line(line, &context.syntax_set).map(|line| {
                        Spans(
                            line.iter()
                                .map(|(style, text)| {
                                    let s = Style::default()
                                        .fg(color(&style.foreground))
                                        .bg(color(&style.background));
                                    if style
                                        .font_style
                                        .contains(FontStyle::BOLD)
                                    {
                                        s.add_modifier(Modifier::BOLD);
                                    }
                                    if style
                                        .font_style
                                        .contains(FontStyle::ITALIC)
                                    {
                                        s.add_modifier(Modifier::ITALIC);
                                    }
                                    if style
                                        .font_style
                                        .contains(FontStyle::UNDERLINE)
                                    {
                                        s.add_modifier(Modifier::UNDERLINED);
                                    }

                                    Span::styled(text.to_string() + "\n", s)
                                })
                                .collect(),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>();
            let text = match lines {
                Ok(lines) => Text { lines },
                Err(_) => Text::raw(text),
            };
            target.push(Section::Code { text });
        }

        NodeValue::FrontMatter(_) => {}

        NodeValue::FootnoteDefinition(footnote) => {
            let name = String::from_utf8_lossy(footnote);
            let mut content = Vec::new();
            sections(
                context,
                source,
                &mut content,
                style.add_modifier(Modifier::DIM),
            );
            let content = content.into();
            context.footnotes.definition(&name, content);
        }

        NodeValue::Heading(heading) => {
            let prefix = match heading.level {
                1 => context.configuration.heading_style1.prefix.clone(),
                2 => context.configuration.heading_style2.prefix.clone(),
                3 => context.configuration.heading_style3.prefix.clone(),
                4 => context.configuration.heading_style4.prefix.clone(),
                5 => context.configuration.heading_style1.prefix.clone(),
                6 => context.configuration.heading_style6.prefix.clone(),
                n => panic!("unexpected level: {}", n),
            };
            let header_style = match heading.level {
                1 => &context.configuration.heading_style1.style,
                2 => &context.configuration.heading_style2.style,
                3 => &context.configuration.heading_style3.style,
                4 => &context.configuration.heading_style4.style,
                5 => &context.configuration.heading_style1.style,
                6 => &context.configuration.heading_style6.style,
                n => panic!("unexpected level: {}", n),
            };
            let mut text = Spans::from(root_inlines(
                context,
                source.children(),
                header_style.into(),
            ));
            text.0.insert(0, prefix.into());
            let level = heading.level as u8;
            target.push(Section::Heading { text, level });
        }

        NodeValue::Item(item) => {
            let mut content = Vec::new();
            sections(context, source, &mut content, style);
            let content = Sections::from(content);
            target.push(match item.list_type {
                ListType::Ordered => Section::ListItemOrdered {
                    content,
                    ordinal: 0,
                    delimiter: match item.delimiter {
                        ListDelimType::Period => '.',
                        ListDelimType::Paren => ')',
                    },
                },
                ListType::Bullet => Section::ListItemUnordered {
                    content,
                    bullet: item.bullet_char.into(),
                },
            })
        }

        NodeValue::List(list) => {
            let mut content = Vec::new();
            sections(context, source, &mut content, style);
            let mut content = Sections::from(content);
            content.inner_margin = 0;
            content.list_item_reorder(list.start);
            target.push(Section::List { content });
        }

        NodeValue::Paragraph => {
            let text =
                Spans::from(root_inlines(context, source.children(), style))
                    .into();
            target.push(Section::Paragraph { text });
        }

        NodeValue::Table(_) => {
            target.push(Section::Table { rows: Vec::new() });
            sections(context, source, target, style);
        }

        NodeValue::TableCell => {
            if let Some(row) = target.last_mut().and_then(|s| {
                if let Section::Table { rows, .. } = s {
                    rows.last_mut()
                } else {
                    None
                }
            }) {
                let text = Spans::from(root_inlines(
                    context,
                    source.children(),
                    style,
                ))
                .into();
                row.push(text);
            }
        }

        NodeValue::TableRow(header) => {
            if let Some(Section::Table { rows, .. }) = target.last_mut() {
                rows.push(TableRow::new(*header));
                sections(context, source, target, style);
            }
        }

        NodeValue::ThematicBreak => {
            target.push(Section::ThematicBreak);
        }

        // TODO: Enable description lists and handle them
        NodeValue::DescriptionDetails
        | NodeValue::DescriptionItem(_)
        | NodeValue::DescriptionTerm => {
            unimplemented!(
                "Description lists are not supported, but found on line {}",
                source.data.borrow().start_line,
            )
        }

        // These are not supported
        NodeValue::HtmlBlock(_) => {
            unimplemented!(
                "The element {:?} on line {} is not supported.",
                node,
                source.data.borrow().start_line
            )
        }

        _ => unimplemented!(
            "{:?} was unexpected on line {}",
            node,
            source.data.borrow().start_line,
        ),
    }
}

/// Handles all children of a node as inline elements.
///
/// # Arguments
/// *  `context` - The context used during transform.
/// *  `source` - The element to handle.
/// *  `style` - The current style.
fn root_inlines<'a>(
    context: &mut Context,
    nodes: impl Iterator<Item = &'a Node<'a, RefCell<Ast>>>,
    style: Style,
) -> Vec<Span<'a>> {
    nodes.fold(Vec::new(), |mut target, source| {
        inline(context, source, &mut target, style);
        target
    })
}

/// Handles all children of a node as inline elements.
///
/// # Arguments
/// *  `context` - The context used during transform.
/// *  `source` - The element to handle.
/// *  `target` - A target `Vec` for generated spans.
/// *  `style` - The current style.
fn inlines<'a>(
    context: &mut Context,
    source: &'a Node<'a, RefCell<Ast>>,
    target: &mut Vec<Span<'a>>,
    style: Style,
) {
    for source in source.children() {
        inline(context, source, target, style)
    }
}

/// Handles a single inline element.
///
/// # Arguments
/// *  `context` - The context used during transform.
/// *  `source` - The element to handle.
/// *  `target` - A target `Vec` for generated spans.
/// *  `style` - The current style.
fn inline<'a>(
    context: &mut Context,
    source: &'a Node<'a, RefCell<Ast>>,
    target: &mut Vec<Span<'a>>,
    style: Style,
) {
    use NodeValue::*;
    let node = &source.data.borrow().value;
    match node {
        Code(code) => target.push(Span::raw(
            String::from_utf8_lossy(&code.literal).into_owned(),
        )),

        Emph => {
            inlines(
                context,
                source,
                target,
                style.add_modifier(Modifier::ITALIC),
            );
        }

        FootnoteReference(footnote) => {
            let name = String::from_utf8_lossy(footnote);
            let index = context.footnotes.reference(&name);
            target.push(
                format!("{}", Footnotes::index_to_superscript(index)).into(),
            )
        }

        LineBreak => {
            target.push(Span::raw("\n"));
        }

        Link(link) => {
            inlines(
                context,
                source,
                target,
                style.add_modifier(Modifier::UNDERLINED).fg(Color::Blue),
            );
            target.push(Span::styled(
                format!(" <{}>", String::from_utf8_lossy(&link.url)),
                style,
            ));
        }

        SoftBreak => target.push(Span::raw(" ")),

        Strong => {
            inlines(
                context,
                source,
                target,
                style.add_modifier(Modifier::BOLD),
            );
        }

        Strikethrough => {
            inlines(
                context,
                source,
                target,
                style.add_modifier(Modifier::CROSSED_OUT),
            );
        }

        Text(text) => {
            target.push(Span::styled(
                String::from_utf8_lossy(text).into_owned(),
                style,
            ));
        }

        // TODO: Enable superscript and handle it
        Superscript => {
            unimplemented!(
                "Superscript is are not supported, but found on line {}",
                source.data.borrow().start_line,
            )
        }

        // TODO: Enable task item lists and handle them
        TaskItem(_) => {
            unimplemented!(
                "Task item lists are not supported, but found on line {}",
                source.data.borrow().start_line,
            )
        }

        // These are not supported
        HtmlInline(_) | Image(_) => {
            unimplemented!(
                "The element {:?} on line {} is not supported.",
                node,
                source.data.borrow().start_line
            )
        }

        _ => unimplemented!(
            "{:?} was unexpected on line {}",
            node,
            source.data.borrow().start_line,
        ),
    }
}
