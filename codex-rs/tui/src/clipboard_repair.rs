//! Repairs native Terminal text copies using source-backed rich transcript cells.
//!
//! macOS Terminal consumes `Command-C`, so the TUI cannot observe that key directly. This module
//! watches the pasteboard change counter instead. When copied text uniquely matches a finalized
//! assistant response after display whitespace is ignored, it replaces the clipboard with the
//! corresponding canonical visible-text slice. Canonical text is rendered without viewport
//! wrapping or the assistant `• ` / two-space gutter, so cyan paths remain contiguous and prose
//! keeps logical paragraph boundaries.

use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use crate::history_cell::AgentMarkdownCell;
use crate::history_cell::HistoryCell;
use crate::terminal_hyperlinks::HyperlinkLine;

pub(crate) const POLL_INTERVAL: Duration = Duration::from_millis(/*millis*/ 50);
const MIN_MATCH_CHARACTERS: usize = 8;
const MAX_CLIPBOARD_BYTES: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClipboardRepairDocument {
    text: String,
}

impl ClipboardRepairDocument {
    pub(crate) fn from_lines(lines: &[HyperlinkLine]) -> Self {
        let mut text = String::new();

        for (line_index, line) in lines.iter().enumerate() {
            if line_index > 0 {
                text.push('\n');
            }
            for span in &line.line.spans {
                text.push_str(span.content.as_ref());
            }
        }

        while text.ends_with('\n') {
            text.pop();
        }
        Self { text }
    }
}

pub(crate) fn documents_from_history(
    cells: &[Arc<dyn HistoryCell>],
) -> Vec<ClipboardRepairDocument> {
    cells
        .iter()
        .filter_map(|cell| cell.as_any().downcast_ref::<AgentMarkdownCell>())
        .map(|cell| ClipboardRepairDocument::from_lines(&cell.clipboard_repair_lines()))
        .filter(|document| !document.text.trim().is_empty())
        .collect()
}

/// Recover the canonical selected range when `copied` has one unique transcript match.
///
/// Terminal-inserted newlines, the assistant gutter, and continuation indentation are display
/// artifacts, so matching ignores whitespace. The returned slice always comes from the canonical
/// source-backed render and therefore restores its original spaces and logical newlines.
pub(crate) fn repair_copied_text(
    copied: &str,
    documents: &[ClipboardRepairDocument],
) -> Option<String> {
    if copied.len() > MAX_CLIPBOARD_BYTES {
        return None;
    }
    let copied = strip_leading_agent_bullet(copied);
    let needle = compact_text(copied.as_ref());
    if needle.characters.len() < MIN_MATCH_CHARACTERS {
        return None;
    }

    let mut candidate = None;
    for document in documents {
        let haystack = compact_text(&document.text);
        if needle.characters.len() > haystack.characters.len() {
            continue;
        }
        for start in 0..=haystack.characters.len() - needle.characters.len() {
            let end = start + needle.characters.len();
            if haystack.characters[start..end] != needle.characters {
                continue;
            }
            if candidate.is_some() {
                return None;
            }
            let source_start = haystack.source_ranges[start].start;
            let source_end = haystack.source_ranges[end - 1].end;
            candidate = Some(document.text[source_start..source_end].to_string());
        }
    }
    candidate
}

struct CompactText {
    characters: Vec<char>,
    source_ranges: Vec<Range<usize>>,
}

fn compact_text(text: &str) -> CompactText {
    let mut characters = Vec::new();
    let mut source_ranges = Vec::new();
    for (start, character) in text.char_indices() {
        if character.is_whitespace() {
            continue;
        }
        characters.push(character);
        source_ranges.push(start..start + character.len_utf8());
    }
    CompactText {
        characters,
        source_ranges,
    }
}

fn strip_leading_agent_bullet(text: &str) -> std::borrow::Cow<'_, str> {
    let Some(first_non_whitespace) = text.find(|character: char| !character.is_whitespace()) else {
        return std::borrow::Cow::Borrowed(text);
    };
    let remainder = &text[first_non_whitespace..];
    let Some(after_bullet) = remainder.strip_prefix('•') else {
        return std::borrow::Cow::Borrowed(text);
    };
    if !after_bullet.chars().next().is_some_and(char::is_whitespace) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut stripped = String::with_capacity(text.len());
    stripped.push_str(&text[..first_non_whitespace]);
    stripped.push_str(after_bullet);
    std::borrow::Cow::Owned(stripped)
}

pub(crate) struct ClipboardRepairMonitor {
    #[cfg(target_os = "macos")]
    clipboard: Option<arboard::Clipboard>,
    #[cfg(target_os = "macos")]
    last_change_count: isize,
}

impl ClipboardRepairMonitor {
    pub(crate) fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            let last_change_count = macos_change_count();
            let clipboard = match crate::clipboard_copy::new_macos_clipboard() {
                Ok(clipboard) => Some(clipboard),
                Err(error) => {
                    tracing::debug!("clipboard repair disabled: {error}");
                    None
                }
            };
            Self {
                clipboard,
                last_change_count,
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {}
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.clipboard.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    pub(crate) fn poll(&mut self, documents: impl FnOnce() -> Vec<ClipboardRepairDocument>) {
        #[cfg(target_os = "macos")]
        {
            let change_count = macos_change_count();
            if change_count == self.last_change_count {
                return;
            }
            self.last_change_count = change_count;

            let Some(clipboard) = self.clipboard.as_mut() else {
                return;
            };
            let Ok(copied) = clipboard.get_text() else {
                return;
            };
            let documents = documents();
            let Some(repaired) = repair_copied_text(&copied, &documents) else {
                return;
            };
            if repaired == copied || macos_change_count() != change_count {
                return;
            }
            if let Err(error) = clipboard.set_text(repaired) {
                tracing::debug!("clipboard repair write failed: {error}");
            }
            self.last_change_count = macos_change_count();
        }
        #[cfg(not(target_os = "macos"))]
        {
            drop(documents);
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_change_count() -> isize {
    let pasteboard = objc2_app_kit::NSPasteboard::generalPasteboard();
    pasteboard.changeCount()
}

#[cfg(test)]
#[path = "clipboard_repair_tests.rs"]
mod tests;
