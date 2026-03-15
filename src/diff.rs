use console::Style;
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Classic unified diff (-u / default)
    Unified,
    /// Side-by-side (future: not yet implemented, reserved)
    #[allow(dead_code)]
    SideBySide,
}

pub struct DiffOptions {
    pub format: OutputFormat,
    pub context_lines: usize,
    pub color: bool,
    pub label_a: String,
    pub label_b: String,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            format: OutputFormat::Unified,
            context_lines: 3,
            color: true,
            label_a: "a".into(),
            label_b: "b".into(),
        }
    }
}

/// Produce a diff string between two text blobs and return it.
/// Returns `None` when the texts are identical.
pub fn diff_texts(old: &str, new: &str, opts: &DiffOptions) -> Option<String> {
    if old == new {
        return None;
    }

    let td = TextDiff::from_lines(old, new);

    let red = Style::new().red();
    let green = Style::new().green();
    let cyan = Style::new().cyan();
    let dim = Style::new().dim();

    let mut output = String::new();

    match opts.format {
        OutputFormat::Unified => {
            // Header
            let header_a = format!("--- {}\n", opts.label_a);
            let header_b = format!("+++ {}\n", opts.label_b);
            if opts.color {
                output.push_str(&red.apply_to(&header_a).to_string());
                output.push_str(&green.apply_to(&header_b).to_string());
            } else {
                output.push_str(&header_a);
                output.push_str(&header_b);
            }

            for group in td.grouped_ops(opts.context_lines) {
                // Hunk header
                let first = group.first().unwrap();
                let _last = group.last().unwrap();
                let old_start = first.old_range().start + 1;
                let old_len: usize = group.iter().map(|op| op.old_range().len()).sum();
                let new_start = first.new_range().start + 1;
                let new_len: usize = group.iter().map(|op| op.new_range().len()).sum();
                let hunk = format!(
                    "@@ -{},{} +{},{} @@\n",
                    old_start, old_len, new_start, new_len
                );
                if opts.color {
                    output.push_str(&cyan.apply_to(&hunk).to_string());
                } else {
                    output.push_str(&hunk);
                }

                for op in &group {
                    for change in td.iter_changes(op) {
                        let sign = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        let line = format!("{}{}", sign, change.value());
                        let formatted = if opts.color {
                            match change.tag() {
                                ChangeTag::Delete => red.apply_to(&line).to_string(),
                                ChangeTag::Insert => green.apply_to(&line).to_string(),
                                ChangeTag::Equal => dim.apply_to(&line).to_string(),
                            }
                        } else {
                            line
                        };
                        output.push_str(&formatted);
                    }
                }
            }
        }
        OutputFormat::SideBySide => {
            // Placeholder — emit unified for now
            output.push_str("(side-by-side not yet implemented)\n");
        }
    }

    Some(output)
}

/// Return exit code: 0 = identical, 1 = different.
pub fn exit_code(diff: &Option<String>) -> i32 {
    match diff {
        None => 0,
        Some(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_no_color() -> DiffOptions {
        DiffOptions {
            color: false,
            ..Default::default()
        }
    }

    #[test]
    fn identical_texts_return_none() {
        let opts = default_no_color();
        assert_eq!(diff_texts("hello\n", "hello\n", &opts), None);
    }

    #[test]
    fn deletion_shown_with_minus() {
        let opts = default_no_color();
        let result = diff_texts("line1\nline2\n", "line1\n", &opts).unwrap();
        assert!(result.contains("-line2"));
    }

    #[test]
    fn insertion_shown_with_plus() {
        let opts = default_no_color();
        let result = diff_texts("line1\n", "line1\nline2\n", &opts).unwrap();
        assert!(result.contains("+line2"));
    }

    #[test]
    fn unified_header_present() {
        let opts = DiffOptions {
            color: false,
            label_a: "file_a.txt".into(),
            label_b: "file_b.txt".into(),
            ..Default::default()
        };
        let result = diff_texts("a\n", "b\n", &opts).unwrap();
        assert!(result.contains("--- file_a.txt"));
        assert!(result.contains("+++ file_b.txt"));
    }

    #[test]
    fn exit_code_0_for_identical() {
        assert_eq!(exit_code(&None), 0);
    }

    #[test]
    fn exit_code_1_for_diff() {
        assert_eq!(exit_code(&Some("something".into())), 1);
    }
}
