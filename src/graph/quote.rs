//! One item's own source, lexed into runs. A run says what it *is* — keyword,
//! string, a reference and where it resolved to — and never how it is inked:
//! the token classes are a lexer's, not a palette's.

use serde::{Deserialize, Serialize};

/// What one run of source text is, for colouring. The classes are a lexer's,
/// not a palette's: the client decides how each one is inked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Tok {
    /// A rust keyword.
    Kw,
    Comment,
    /// A doc comment: `///`, `//!`, `/** */`.
    Doc,
    /// A string, char, or byte literal.
    Str,
    Num,
    Lifetime,
    /// Anything inside an attribute, `#[derive(Clone)]` included.
    Attr,
    /// A name whose first letter is uppercase.
    Type,
    /// The name in a `fn` declaration.
    Fn,
    /// A macro name, called or declared.
    Macro,
    Ident,
    Punct,
    Space,
}

/// One run of quoted source: its text, its colour class, and — when the run
/// is a resolved reference to something in the workspace — where it goes, as
/// an index into [`ItemSource::links`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SrcRun {
    pub(crate) text: String,
    pub(crate) tok: Tok,
    pub(crate) link: Option<u32>,
}

/// Where a clickable run of quoted source goes: the item it resolved to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SrcLink {
    /// Target file path relative to the workspace root.
    pub(crate) path: String,
    /// The target's [`super::data::ItemMark::label`] — `Type::method` inside a section,
    /// the plain name otherwise. Empty when the reference names the file as a
    /// whole (a `use` of its module), which this chart cannot go to.
    pub(crate) label: String,
}

/// One run of quoted lines, and where in the file they stand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SrcBlock {
    /// 1-based line the first of these lines is, in the real file.
    pub(crate) first_line: u32,
    /// Per line, its runs of text in order. A run whose name resolved to
    /// something in the workspace carries a link.
    pub(crate) lines: Vec<Vec<SrcRun>>,
}

/// One item's own source text, lexed into coloured runs — what Go to
/// Definition lands on. The interface quotes the file rather than describing
/// it, so nothing here is reconstructed: the runs concatenate back to exactly
/// the bytes on disk, minus the indent the outermost quoted block was
/// stripped of.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ItemSource {
    /// Path relative to the workspace root, for the locator.
    pub(crate) path: String,
    /// What to draw, in source order, each block keeping the indent it is
    /// written at: the `impl` or `trait` header the item sits under, the item
    /// itself, then the brace that closes that block. A free item is one
    /// block on its own.
    ///
    /// The header is not decoration: an associated item's span holds none of
    /// it — the block is its own item — so a method quoted alone reads as a
    /// free function that could not compile (`fn edge_style(self, …)` is not
    /// rust) and says nothing about whose method it is. Whatever the file
    /// writes between two blocks is not quoted, and the client says so rather
    /// than closing the gap silently.
    pub(crate) blocks: Vec<SrcBlock>,
    /// The navigation targets the runs link to, deduplicated. Every block
    /// indexes into this one list.
    pub(crate) links: Vec<SrcLink>,
}
