use std::cell::RefCell;
use std::ops::Deref;

use comrak::arena_tree::Node;
use comrak::nodes::{Ast, ListDelimType, ListType, NodeValue};

use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};

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

impl<'a> From<&'a Page<'a>> for Sections<'a> {
    fn from(source: &'a Page<'a>) -> Self {
        {
            let mut sections = Vec::new();
            for source in source.nodes() {
                section(source, &mut sections, Style::default());
            }
            sections.into()
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

    /// A thematic break
    ThematicBreak,
}

impl<'a> Section<'a> {
    /// The number of cells each level of indentaion provides.
    pub const INDENT: u16 = 4;
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
            let content = content.into();
            target.push(Section::BlockQuote { content });
        }

        NodeValue::CodeBlock(code) => {
            // TODO: Apply highlight based on code.info
            let text = Text {
                lines: String::from_utf8_lossy(&code.literal)
                    .into_owned()
                    .split('\n')
                    .map(|s| s.to_string().into())
                    .collect::<Vec<_>>(),
            };
            target.push(Section::Code { text });
        }

        NodeValue::FrontMatter(_) => {}

        NodeValue::Heading(heading) => {
            let text = Spans::from(root_inlines(
                source.children(),
                style.add_modifier(Modifier::UNDERLINED),
            ))
            .into();
            let level = heading.level as u8;
            target.push(Section::Heading { text, level });
        }

        NodeValue::Item(item) => {
            let mut content = Vec::new();
            sections(source, &mut content, style);
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
            sections(source, &mut content, style);
            let mut content = Sections::from(content);
            content.inner_margin = 0;
            content.list_item_reorder(list.start);
            target.push(Section::List { content });
        }

        NodeValue::Paragraph => {
            let text =
                Spans::from(root_inlines(source.children(), style)).into();
            target.push(Section::Paragraph { text });
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

        // TODO: Enable footnote references and handle them
        NodeValue::FootnoteDefinition(_) => {
            unimplemented!(
                "Footnote definitions are not supported, but found on line {}",
                source.data.borrow().start_line,
            )
        }

        // TODO: Enable tables and handle them
        NodeValue::TableCell | NodeValue::TableRow(_) | NodeValue::Table(_) => {
            unimplemented!(
                "Tables are not supported, but found on line {}",
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

        // TODO: Enable footnote references and handle them
        FootnoteReference(_) => {
            unimplemented!(
                "Footnote references are not supported, but found on line {}",
                source.data.borrow().start_line,
            )
        }

        // TODO: Enable strikethrough and handle it
        Strikethrough => {
            unimplemented!(
                "Strikethrough is not supported, but found on line {}",
                source.data.borrow().start_line,
            )
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
