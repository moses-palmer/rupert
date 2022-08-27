use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::Path;

use comrak::arena_tree::Node;
use comrak::nodes::Ast;
use comrak::Arena;

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
            &comrak::ComrakOptions::default(),
        ),
    })
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
}
