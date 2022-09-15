use std::cell::RefCell;

use comrak::arena_tree::Node;
use comrak::nodes::{Ast, ListDelimType, ListType, NodeValue};

use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};

use crate::presentation::Page;

/// A collection of sections.
#[derive(Clone)]
pub struct Sections<'a>(Vec<Section<'a>>);

impl<'a> Sections<'a> {
    /// Reorders all ordered list items in a list of sections.
    ///
    /// # Arguments
    /// *  `start_at` - The starting index.
    fn list_item_reorder(&mut self, start_at: usize) {
        self.0
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

    /// Produces an iterator over all sections.
    pub fn iter(&'a self) -> impl Iterator<Item = &'a Section<'a>> {
        self.0.iter()
    }

    /// The number of sections.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> From<&'a Page<'a>> for Sections<'a> {
    fn from(source: &'a Page<'a>) -> Self {
        {
            let mut sections = Vec::new();
            for source in source.nodes() {
                section(source, &mut sections, Style::default());
            }
            Self(sections)
        }
    }
}

/// A page section.
#[derive(Clone)]
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

    /// A thematic break
    ThematicBreak,
}

impl<'a> Section<'a> {
    /// The number of cells each level of indentaion provides.
    pub const INDENT: u16 = 4;

    /// Creates a block quote.
    ///
    /// # Arguments
    /// *  `content` - The content.
    pub fn block_quote(content: Sections<'a>) -> Self {
        Self::BlockQuote { content }
    }

    /// Creates a code section.
    ///
    /// # Arguments
    /// *  `text` - The text.
    pub fn code(text: Text<'a>) -> Self {
        Self::Code { text }
    }

    /// Creates a heading section.
    ///
    /// # Arguments
    /// *  `text` - The text.
    /// *  `level` - The heading level.
    pub fn heading(text: Spans<'a>, level: u8) -> Self {
        Self::Heading { text, level }
    }

    /// Creates an ordered list item section.
    ///
    /// # Arguments
    /// *  `content` - The content.
    /// *  `ordinal` - The item ordinal.
    /// *  `delimiter` - The delimiter displayed after the numeral.
    pub fn list_item_ordered(
        content: Sections<'a>,
        ordinal: usize,
        delimiter: char,
    ) -> Self {
        Self::ListItemOrdered {
            content,
            ordinal,
            delimiter,
        }
    }

    /// Creates an unordered list item section.
    ///
    /// # Arguments
    /// *  `content` - The content.
    /// *  `bullet` - The character used as marker.
    pub fn list_item_unordered(content: Sections<'a>, bullet: char) -> Self {
        Self::ListItemUnordered { content, bullet }
    }

    /// Creates a paragraph section.
    ///
    /// # Arguments
    /// *  `text` - The text.
    pub fn paragraph(text: Text<'a>) -> Self {
        Self::Paragraph { text }
    }

    /// Creates a thematic break.
    pub fn thematic_break() -> Self {
        Self::ThematicBreak
    }
}

/// Converts a collection of markdown AST nodes to sections.
///
/// # Arguments
/// *  `nodes` - The nodes to style.
/// *  `style` - The current style.
fn sections<'a>(
    source: &'a Node<'a, RefCell<Ast>>,
    target: &mut Vec<Section<'a>>,
    style: Style,
) {
    for source in source.children() {
        section(source, target, style);
    }
}

/// Handles a single block element.
///
/// # Arguments
/// *  `source` - The element to handle.
/// *  `target` - A target `Vec` for generated spans.
/// *  `style` - The current style.
fn section<'a>(
    source: &'a Node<'a, RefCell<Ast>>,
    target: &mut Vec<Section<'a>>,
    style: Style,
) {
    let node = &source.data.borrow().value;
    match node {
        NodeValue::BlockQuote => {
            let mut content = Vec::new();
            sections(source, &mut content, style.add_modifier(Modifier::DIM));
            let content = Sections(content);
            target.push(Section::block_quote(content));
        }

        NodeValue::CodeBlock(code) => {
            // TODO: Apply highlight based on code.info
            target.push(Section::code(Text {
                lines: String::from_utf8_lossy(&code.literal)
                    .into_owned()
                    .split('\n')
                    .map(|s| s.to_string().into())
                    .collect::<Vec<_>>(),
            }));
        }

        NodeValue::Heading(heading) => {
            if !target.is_empty() {
                target.push(Section::paragraph("".into()));
            }
            target.push(Section::heading(
                Spans::from(root_inlines(
                    source.children(),
                    style.add_modifier(Modifier::UNDERLINED),
                ))
                .into(),
                heading.level as u8,
            ));
        }

        NodeValue::Item(item) => {
            let mut content = Vec::new();
            sections(source, &mut content, style);
            let content = Sections(content);
            target.push(match item.list_type {
                ListType::Ordered => Section::list_item_ordered(
                    content,
                    0,
                    match item.delimiter {
                        ListDelimType::Period => '.',
                        ListDelimType::Paren => ')',
                    },
                ),
                ListType::Bullet => Section::list_item_unordered(
                    content,
                    item.bullet_char.into(),
                ),
            })
        }

        NodeValue::List(list) => {
            let mut content = Vec::new();
            sections(source, &mut content, style);
            let mut content = Sections(content);
            content.list_item_reorder(list.start);
            target.extend(content.0);
        }

        NodeValue::Paragraph => {
            target.push(Section::paragraph(
                Spans::from(root_inlines(source.children(), style)).into(),
            ));
        }
        NodeValue::ThematicBreak => {
            target.push(Section::thematic_break());
        }

        // TODO: Enable tables and handle them
        NodeValue::TableCell | NodeValue::TableRow(_) | NodeValue::Table(_) => {
            unimplemented!(
                "Tables are not supported, but found on line {}",
                source.data.borrow().start_line,
            )
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
        NodeValue::FootnoteDefinition(_) | NodeValue::HtmlBlock(_) => {
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
/// *  `source` - The element to handle.
/// *  `style` - The current style.
fn root_inlines<'a>(
    nodes: impl Iterator<Item = &'a Node<'a, RefCell<Ast>>>,
    style: Style,
) -> Vec<Span<'a>> {
    nodes.fold(Vec::new(), |mut target, source| {
        inline(source, &mut target, style);
        target
    })
}

/// Handles all children of a node as inline elements.
///
/// # Arguments
/// *  `source` - The element to handle.
/// *  `target` - A target `Vec` for generated spans.
/// *  `style` - The current style.
fn inlines<'a>(
    source: &'a Node<'a, RefCell<Ast>>,
    target: &mut Vec<Span<'a>>,
    style: Style,
) {
    for source in source.children() {
        inline(source, target, style)
    }
}

/// Handles a single inline element.
///
/// # Arguments
/// *  `source` - The element to handle.
/// *  `target` - A target `Vec` for generated spans.
/// *  `style` - The current style.
fn inline<'a>(
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
            inlines(source, target, style.add_modifier(Modifier::ITALIC));
        }

        LineBreak => {
            target.push(Span::raw("\n"));
        }

        Link(link) => {
            inlines(
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
            inlines(source, target, style.add_modifier(Modifier::BOLD));
        }

        Text(text) => {
            target.push(Span::styled(
                String::from_utf8_lossy(text).into_owned(),
                style,
            ));
        }

        // TODO: Enable strikethrough and handle it
        Strikethrough => {
            unimplemented!(
                "Strikethrough is not supported, but found on line {}",
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
        FootnoteReference(_) | HtmlInline(_) | Image(_) | Superscript => {
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
