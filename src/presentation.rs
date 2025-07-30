use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::Path;

use comrak::Arena;
use comrak::arena_tree::Node;
use comrak::nodes::{Ast, NodeValue};
use serde::{Deserialize, Serialize};

use crate::configuration::ConfigurationFragment;

/// The delimiter used for the front matter.
const FRONT_MATTER_DELIMITER: &str = "%%%";

/// A presentation.
pub struct Presentation<'a> {
    /// The root of the AST.
    root: &'a Node<'a, RefCell<Ast>>,
}

/// Loads a markdown document.
///
/// # Arguments
/// *  `arena` - The arena managing memory for the AST.
/// *  `path` - The path to the document.
pub fn load<'a, P>(
    arena: &'a Arena<Node<'a, RefCell<Ast>>>,
    path: P,
) -> io::Result<Presentation<'a>>
where
    P: AsRef<Path>,
{
    fs::read_to_string(path).map(|data| Presentation {
        root: comrak::parse_document(
            arena,
            &data,
            &comrak::ComrakOptions {
                extension: comrak::ComrakExtensionOptions {
                    footnotes: true,
                    front_matter_delimiter: Some(FRONT_MATTER_DELIMITER.into()),
                    strikethrough: true,
                    table: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        ),
    })
}

impl<'a> Presentation<'a> {
    /// Attempts to load a configuration fragment from the presentation file.
    ///
    /// The configuration is specified as front matter, with `"%%%"` as
    /// delimiter. Only
    pub fn configuration(&self) -> Option<io::Result<ConfigurationFragment>> {
        self.root
            .children()
            .find_map(|node| match &node.data.borrow().value {
                NodeValue::FrontMatter(data) => String::from_utf8(data.clone())
                    .ok()
                    .filter(|s| s.len() > 2 * FRONT_MATTER_DELIMITER.len()),
                _ => None,
            })
            .map(|s| {
                toml::from_str(
                    &s[FRONT_MATTER_DELIMITER.len()
                        ..s.len() - FRONT_MATTER_DELIMITER.len() - 1],
                )
                .map_err(io::Error::other)
            })
    }

    /// The pages of this presentation.
    ///
    /// # Arguments
    /// *  `break_condition` - The break condition for breaking the full
    ///    document into pages.
    pub fn pages(
        &self,
        break_condition: PageBreakCondition,
    ) -> impl Iterator<Item = Page<'a>> {
        PageIterator::new(self, break_condition)
    }
}

/// A single page of the presentation.
pub struct Page<'a> {
    /// The nodes of the AST.
    nodes: Vec<&'a Node<'a, RefCell<Ast>>>,
}

impl<'a> From<Vec<&'a Node<'a, RefCell<Ast>>>> for Page<'a> {
    fn from(source: Vec<&'a Node<'a, RefCell<Ast>>>) -> Self {
        Self { nodes: source }
    }
}

impl<'a> Page<'a> {
    /// An iterator over the AST nodes of this page.
    pub fn nodes(&'a self) -> impl Iterator<Item = &'a Node<'a, RefCell<Ast>>> {
        self.nodes.iter().cloned()
    }
}

/// Conditions for breaking a document into pages.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PageBreakCondition {
    /// Break on thematic break.
    ThematicBreak,

    /// Break on headings.
    Heading {
        /// The heading level.
        level: u32,
    },
}

impl PageBreakCondition {
    /// Determines whether a node value signifies a page break.
    ///
    /// # Arguments
    /// *  `value` - The node value to check.
    pub fn try_break<'a>(
        &self,
        node: &'a Node<'a, RefCell<Ast>>,
    ) -> Option<&'a Node<'a, RefCell<Ast>>> {
        use PageBreakCondition::*;
        match self {
            ThematicBreak => match node.data.borrow().value {
                NodeValue::ThematicBreak => node.next_sibling(),
                _ => None,
            },
            Heading { level } => match node.data.borrow().value {
                NodeValue::Heading(h) if h.level == *level => Some(node),
                _ => None,
            },
        }
    }
}

/// An iterator over pages.
struct PageIterator<'a> {
    /// The page break condition.
    break_condition: PageBreakCondition,

    /// The next node.
    next: Option<&'a Node<'a, RefCell<Ast>>>,
}

impl<'a> PageIterator<'a> {
    pub fn new(
        presentation: &Presentation<'a>,
        break_condition: PageBreakCondition,
    ) -> Self {
        Self {
            next: presentation.root.first_child(),
            break_condition,
        }
    }
}

impl<'a> Iterator for PageIterator<'a> {
    type Item = Page<'a>;

    fn next(&mut self) -> Option<Page<'a>> {
        let mut current = self.next?;
        let mut nodes = Vec::new();
        self.next = loop {
            if let NodeValue::FrontMatter(_) = &current.data.borrow().value {
                current = current.next_sibling()?;
                continue;
            }

            nodes.push(current);
            if let Some(next_sibling) = current.next_sibling() {
                if let Some(next) = self.break_condition.try_break(next_sibling)
                {
                    break Some(next);
                } else {
                    current = next_sibling;
                    continue;
                }
            } else {
                break None;
            }
        };

        Some(nodes.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_sucessful() {
        let mut arena = comrak::Arena::new();
        let presentation = load(&mut arena, "test-resources/presentation.md");

        assert!(presentation.is_ok());
    }

    #[test]
    fn load_fails_for_nonexisting() {
        let mut arena = comrak::Arena::new();
        let presentation = load(&mut arena, "test-resources/does-not-exist.md");

        assert!(presentation.is_err());
    }

    #[test]
    fn pages() {
        let mut arena = comrak::Arena::new();
        let presentation =
            load(&mut arena, "test-resources/presentation.md").unwrap();

        let pages = presentation
            .pages(PageBreakCondition::Heading { level: 1 })
            .collect::<Vec<_>>();

        assert_eq!(2, pages.len());
        assert_eq!(1, pages[0].nodes[0].data.borrow().start_line);
        assert_eq!(6, pages[1].nodes[0].data.borrow().start_line);
    }
}
