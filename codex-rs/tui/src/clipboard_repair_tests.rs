use super::*;
use crate::history_cell::AgentMarkdownCell;
use crate::history_cell::HistoryCell;
use pretty_assertions::assert_eq;
use std::path::Path;

fn document(markdown: &str) -> ClipboardRepairDocument {
    let cell = AgentMarkdownCell::new(markdown.to_string(), Path::new("/tmp"));
    ClipboardRepairDocument::from_lines(&cell.clipboard_repair_lines())
}

fn plain_text(lines: &[HyperlinkLine]) -> String {
    lines
        .iter()
        .map(|line| {
            line.line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn normal_prose_restores_paragraphs_and_removes_display_gutter() {
    let source =
        "First paragraph wraps onto another visual row.\n\nSecond paragraph stays separate.";
    let copied = "• First paragraph wraps onto\n  another visual row.\n  \n  Second paragraph stays\n  separate.";

    assert_eq!(
        repair_copied_text(copied, &[document(source)]),
        Some(source.to_string())
    );
}

#[test]
fn rich_viewport_rows_round_trip_to_logical_paragraphs() {
    let source = "\
Microscopes are expensive, limiting access in schools and healthcare. I saw this in my high school biology lab, where our microscopes were outdated, underperforming, and overpriced.

Each iteration therefore addressed a different limitation. I moved from proving the imaging principle, to integrating the mechanical system, to improving tolerancing, repeatability, assembly, and scalability.

I learned that building does not end when the first prototype works. The real challenge is continuing to iterate until a technical idea can work reliably outside the hands of the person who created it.";
    let cell = AgentMarkdownCell::new(source.to_string(), Path::new("/tmp"));
    let copied = plain_text(&cell.display_hyperlink_lines(/*width*/ 54));

    assert!(copied.starts_with("• "));
    assert!(copied.contains("\n  "));
    assert_eq!(
        repair_copied_text(&copied, &[document(source)]),
        Some(source.to_string())
    );
}

#[test]
fn cyan_path_removes_visual_wrap_without_adding_a_space() {
    let path = "/Users/williamxu/Desktop/Projects/codex/codex-rs/tui/src/app.rs";
    let markdown = format!("Open `{path}`.");
    let cell = AgentMarkdownCell::new(markdown.clone(), Path::new("/tmp"));
    let lines = cell.clipboard_repair_lines();
    assert!(lines.iter().flat_map(|line| &line.line.spans).any(|span| {
        matches!(
            span.style.patch(ratatui::style::Style::default()).fg,
            Some(
                ratatui::style::Color::Blue
                    | ratatui::style::Color::LightBlue
                    | ratatui::style::Color::Cyan
                    | ratatui::style::Color::LightCyan
            )
        ) && span.content.contains(path)
    }));

    let copied = "Open /Users/williamxu/Desktop/Projects/codex/\n  codex-rs/tui/src/app.rs.";
    assert_eq!(
        repair_copied_text(copied, &[document(&markdown)]),
        Some(format!("Open {path}."))
    );
}

#[test]
fn mixed_prose_path_and_paragraph_selection_uses_canonical_boundaries() {
    let path = "/Users/williamxu/Desktop/Projects/codex/README.md";
    let markdown = format!("Read `{path}` before continuing.\n\nThen run the normal workflow.");
    let copied = "Read /Users/williamxu/Desktop/Projects/\n  codex/README.md before continuing.\n  \n  Then run the normal\n  workflow.";

    assert_eq!(
        repair_copied_text(copied, &[document(&markdown)]),
        Some(format!(
            "Read {path} before continuing.\n\nThen run the normal workflow."
        ))
    );
}

#[test]
fn partial_selection_recovers_only_the_selected_canonical_range() {
    let copied = "paragraph wraps onto\n  another visual row";
    assert_eq!(
        repair_copied_text(
            copied,
            &[document(
                "First paragraph wraps onto another visual row.\n\nSecond paragraph."
            )]
        ),
        Some("paragraph wraps onto another visual row".to_string())
    );
}

#[test]
fn ambiguous_or_unrelated_clipboard_text_is_not_changed() {
    let duplicate = document("A unique-looking repeated sentence appears here.");
    assert_eq!(
        repair_copied_text(
            "unique-looking repeated\n  sentence",
            &[duplicate.clone(), duplicate]
        ),
        None
    );
    assert_eq!(
        repair_copied_text(
            "text copied from another application",
            &[document("Codex text")]
        ),
        None
    );
}
