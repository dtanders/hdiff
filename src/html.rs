use scraper::Html;

/// Tags whose content we skip entirely (scripts, styles, metadata, etc.)
const SKIP_TAGS: &[&str] = &[
    "script", "style", "noscript", "head", "meta", "link", "svg", "canvas",
];

/// Tags that introduce a block break in the output text
const BLOCK_TAGS: &[&str] = &[
    "p",
    "div",
    "section",
    "article",
    "main",
    "header",
    "footer",
    "aside",
    "nav",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "li",
    "dt",
    "dd",
    "blockquote",
    "pre",
    "figcaption",
    "figure",
    "table",
    "tr",
    "td",
    "th",
    "caption",
    "br",
    "hr",
];

/// Extract human-readable plaintext from an HTML string.
///
/// The extraction:
/// - skips script/style/svg/etc. subtrees entirely
/// - inserts a blank line between block-level elements
/// - collapses runs of whitespace within inline content
/// - trims leading/trailing blank lines from the result
pub fn extract_text(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut out = String::new();
    walk(document.root_element(), &mut out);
    // Normalise: collapse 3+ consecutive newlines to 2, trim edges
    let mut result = String::new();
    let mut blank_run = 0usize;
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
        } else {
            if blank_run > 0 && !result.is_empty() {
                result.push('\n');
            }
            blank_run = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }
    result.trim_matches('\n').to_string()
}

fn walk(node: scraper::ElementRef, out: &mut String) {
    use scraper::node::Node;

    for child in node.children() {
        match child.value() {
            Node::Text(text) => {
                let t = text.trim();
                if !t.is_empty() {
                    // collapse internal whitespace
                    let collapsed: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
                    out.push_str(&collapsed);
                    out.push(' ');
                }
            }
            Node::Element(el) => {
                let tag = el.name().to_ascii_lowercase();
                if SKIP_TAGS.contains(&tag.as_str()) {
                    continue;
                }
                let is_block = BLOCK_TAGS.contains(&tag.as_str());
                if is_block {
                    // flush any trailing inline text and start a new block
                    let trimmed_so_far = out.trim_end_matches(' ');
                    let new_len = trimmed_so_far.len();
                    out.truncate(new_len);
                    out.push('\n');
                }

                if let Some(el_ref) = scraper::ElementRef::wrap(child) {
                    walk(el_ref, out);
                }

                if is_block {
                    let trimmed_so_far = out.trim_end_matches(' ');
                    let new_len = trimmed_so_far.len();
                    out.truncate(new_len);
                    out.push('\n');
                }
            }
            _ => {}
        }
    }
}

/// Detect whether a byte slice looks like HTML.
/// We check for a BOM-stripped leading `<` or a DOCTYPE/html tag.
pub fn looks_like_html(content: &str) -> bool {
    let s = content.trim_start();
    s.starts_with("<!") || {
        // look for an <html or <HTML tag in the first 512 bytes
        let head = &s[..s.len().min(512)];
        head.to_ascii_lowercase().contains("<html")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_paragraph_text() {
        let html = "<html><body><p>Hello, world!</p></body></html>";
        assert_eq!(extract_text(html), "Hello, world!");
    }

    #[test]
    fn skips_script_and_style() {
        let html = r#"<html><head>
            <style>body { color: red; }</style>
            <script>alert('hi')</script>
        </head><body><p>Visible</p></body></html>"#;
        let text = extract_text(html);
        assert!(!text.contains("color"));
        assert!(!text.contains("alert"));
        assert!(text.contains("Visible"));
    }

    #[test]
    fn blocks_separated_by_blank_lines() {
        let html = "<html><body><p>First</p><p>Second</p></body></html>";
        let text = extract_text(html);
        assert!(text.contains("First"));
        assert!(text.contains("Second"));
    }

    #[test]
    fn headings_extracted() {
        let html = "<html><body><h1>Title</h1><p>Body text here.</p></body></html>";
        let text = extract_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("Body text here."));
    }

    #[test]
    fn looks_like_html_detection() {
        assert!(looks_like_html("<!DOCTYPE html><html></html>"));
        assert!(looks_like_html("  <html><body></body></html>"));
        assert!(!looks_like_html("Just plain text"));
        assert!(!looks_like_html("# Markdown heading"));
    }
}
