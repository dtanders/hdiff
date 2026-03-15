/// Integration tests for hdiff.
///
/// Test data is either generated inline or written to a tempdir.
/// One test fetches a live URL — it is gated behind the `net_tests` feature
/// so it can be skipped in offline CI:  `cargo test --features net_tests`
use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn binary() -> std::path::PathBuf {
    // Works whether run via `cargo test` or directly
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // deps/
    if p.ends_with("deps") {
        p.pop();
    }
    p.push("hdiff");
    // on Windows the binary has .exe
    if cfg!(windows) {
        p.set_extension("exe");
    }
    p
}

fn hdiff(args: &[&str]) -> std::process::Output {
    Command::new(binary()).args(args).output().expect("hdiff binary not found — run `cargo build` first")
}

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self { dir: TempDir::new().unwrap() }
    }

    fn write(&self, name: &str, content: &str) -> std::path::PathBuf {
        let p = self.dir.path().join(name);
        fs::write(&p, content).unwrap();
        p
    }
}

fn path_str(p: &std::path::Path) -> &str {
    p.to_str().unwrap()
}

// ── plain text vs plain text ─────────────────────────────────────────────────

#[test]
fn text_identical_exits_0() {
    let f = Fixture::new();
    let a = f.write("a.txt", "Hello world\n");
    let b = f.write("b.txt", "Hello world\n");
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
}

#[test]
fn text_different_exits_1() {
    let f = Fixture::new();
    let a = f.write("a.txt", "foo\n");
    let b = f.write("b.txt", "bar\n");
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn text_diff_shows_minus_plus() {
    let f = Fixture::new();
    let a = f.write("a.txt", "line1\nline2\nline3\n");
    let b = f.write("b.txt", "line1\nLINE2\nline3\n");
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("-line2"), "expected -line2 in:\n{stdout}");
    assert!(stdout.contains("+LINE2"), "expected +LINE2 in:\n{stdout}");
}

#[test]
fn brief_flag_prints_differ_message() {
    let f = Fixture::new();
    let a = f.write("a.txt", "alpha\n");
    let b = f.write("b.txt", "beta\n");
    let out = hdiff(&["--no-color", "-q", path_str(&a), path_str(&b)]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("differ"), "expected 'differ' in: {stdout}");
}

#[test]
fn context_lines_flag() {
    let f = Fixture::new();
    // 10 lines, only middle changes — with -U0 no context lines expected
    let a = f.write("a.txt", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
    let b = f.write("b.txt", "1\n2\n3\n4\nX\n6\n7\n8\n9\n10\n");
    let out = hdiff(&["--no-color", "-U", "0", path_str(&a), path_str(&b)]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // With 0 context lines the unchanged lines 4 and 6 must NOT appear
    assert!(!stdout.contains(" 4\n"), "unexpected context line 4:\n{stdout}");
    assert!(!stdout.contains(" 6\n"), "unexpected context line 6:\n{stdout}");
    assert!(stdout.contains("-5"), "expected -5 in:\n{stdout}");
    assert!(stdout.contains("+X"), "expected +X in:\n{stdout}");
}

// ── HTML vs HTML ──────────────────────────────────────────────────────────────

const HTML_V1: &str = r#"<!DOCTYPE html>
<html><head><title>Test page</title>
<style>body { font-size: 14px; }</style>
<script>console.log('irrelevant');</script>
</head>
<body>
  <h1>Welcome</h1>
  <p>This is version one of the document.</p>
  <p>Shared paragraph.</p>
</body></html>"#;

const HTML_V2: &str = r#"<!DOCTYPE html>
<html><head><title>Test page v2</title>
<style>body { font-size: 16px; }</style>
</head>
<body>
  <h1>Welcome</h1>
  <p>This is version TWO of the document.</p>
  <p>Shared paragraph.</p>
</body></html>"#;

#[test]
fn html_identical_exits_0() {
    let f = Fixture::new();
    let a = f.write("a.html", HTML_V1);
    let b = f.write("b.html", HTML_V1);
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn html_style_change_invisible_to_hdiff() {
    // Only the font-size changed in <style> — hdiff should see identical text
    let f = Fixture::new();
    let a = f.write("a.html", HTML_V1);
    let b = f.write("b.html", HTML_V2);
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    // Text differs because "version one" vs "version TWO"
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // CSS content must NOT appear in the diff
    assert!(!stdout.contains("font-size"), "CSS leaked into diff:\n{stdout}");
    // The textual change must appear
    assert!(stdout.contains("one") || stdout.contains("TWO"), "expected content change:\n{stdout}");
}

#[test]
fn html_script_content_not_in_diff() {
    let f = Fixture::new();
    let a = f.write("a.html", HTML_V1);
    let b = f.write(
        "b.html",
        &HTML_V1.replace("console.log('irrelevant');", "console.log('changed');"),
    );
    // Both are still identical from a visible-text perspective
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "script-only change should produce exit 0"
    );
}

// ── HTML vs plain text ────────────────────────────────────────────────────────

#[test]
fn html_vs_plaintext_matching_content() {
    let f = Fixture::new();
    let html = f.write(
        "page.html",
        r#"<!DOCTYPE html><html><body><p>Rust is great.</p></body></html>"#,
    );
    let txt = f.write("plain.txt", "Rust is great.");
    let out = hdiff(&["--no-color", path_str(&html), path_str(&txt)]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn html_vs_plaintext_different_content() {
    let f = Fixture::new();
    let html = f.write(
        "page.html",
        r#"<!DOCTYPE html><html><body><p>Rust is great.</p></body></html>"#,
    );
    let txt = f.write("plain.txt", "Python is great.");
    let out = hdiff(&["--no-color", path_str(&html), path_str(&txt)]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("-Rust") || stdout.contains("+Python"), "{stdout}");
}

// ── force flags ───────────────────────────────────────────────────────────────

#[test]
fn force_text_ignores_html_tags() {
    // Without --text the HTML is parsed; with --text the raw bytes are compared.
    let f = Fixture::new();
    let html = f.write(
        "page.html",
        r#"<!DOCTYPE html><html><body><p>Hello</p></body></html>"#,
    );
    let txt = f.write("plain.txt", "Hello");
    // Without force-text: content matches → exit 0
    let out_auto = hdiff(&["--no-color", path_str(&html), path_str(&txt)]);
    assert_eq!(out_auto.status.code(), Some(0), "auto-detect should match");

    // With --text both files treated as raw text → raw HTML != "Hello" → exit 1
    let out_forced = hdiff(&["--no-color", "--text", path_str(&html), path_str(&txt)]);
    assert_eq!(out_forced.status.code(), Some(1), "--text should see raw tags");
}

// ── multi-line / realistic HTML documents ─────────────────────────────────────

fn make_article(title: &str, paras: &[&str]) -> String {
    let body: String = paras
        .iter()
        .map(|p| format!("  <p>{p}</p>\n"))
        .collect();
    format!(
        "<!DOCTYPE html>\n<html><head><title>{title}</title></head>\n<body>\n<h1>{title}</h1>\n{body}</body></html>\n",
    )
}

#[test]
fn realistic_article_diff() {
    let paras_v1 = [
        "The quick brown fox jumps over the lazy dog.",
        "Pack my box with five dozen liquor jugs.",
        "How vexingly quick daft zebras jump.",
    ];
    let paras_v2 = [
        "The quick brown fox jumps over the lazy dog.",
        "Pack my box with five dozen liquor jugs.",
        "How vexingly quick daft zebras LEAP.",  // changed last word
    ];
    let f = Fixture::new();
    let a = f.write("v1.html", &make_article("Pangrams", &paras_v1));
    let b = f.write("v2.html", &make_article("Pangrams", &paras_v2));
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("jump.") || stdout.contains("LEAP"), "{stdout}");
}

#[test]
fn list_items_extracted() {
    let f = Fixture::new();
    let a = f.write(
        "a.html",
        r#"<!DOCTYPE html><html><body><ul>
            <li>Apple</li><li>Banana</li><li>Cherry</li>
        </ul></body></html>"#,
    );
    let b = f.write(
        "b.html",
        r#"<!DOCTYPE html><html><body><ul>
            <li>Apple</li><li>Blueberry</li><li>Cherry</li>
        </ul></body></html>"#,
    );
    let out = hdiff(&["--no-color", path_str(&a), path_str(&b)]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("-Banana") || stdout.contains("+Blueberry"), "{stdout}");
}

// ── error handling ────────────────────────────────────────────────────────────

#[test]
fn missing_file_exits_2() {
    let out = hdiff(&["--no-color", "nonexistent_a.txt", "nonexistent_b.txt"]);
    assert_eq!(out.status.code(), Some(2));
}

// ── optional network test ─────────────────────────────────────────────────────
// Run with:  cargo test --features net_tests -- net_wikipedia

#[cfg(feature = "net_tests")]
#[test]
fn net_wikipedia_rust_page_html_vs_text() {
    use std::io::Read;

    // Download the Rust (programming language) Wikipedia article as HTML
    let url = "https://en.wikipedia.org/wiki/Rust_(programming_language)";
    let mut resp = ureq::get(url).call().expect("network request failed").into_reader();
    let mut html_bytes = Vec::new();
    resp.read_to_end(&mut html_bytes).unwrap();
    let html = String::from_utf8_lossy(&html_bytes).into_owned();

    let f = Fixture::new();
    let html_file = f.write("rust_wiki.html", &html);

    // Extract text ourselves and save as .txt — then diff HTML vs that .txt → should be identical
    let extracted = hdiff::html::extract_text(&html);
    let txt_file = f.write("rust_wiki.txt", &extracted);

    // The extracted text from the HTML file vs the saved text should match
    let out = hdiff(&[
        "--no-color",
        path_str(&html_file),
        "--html-b",   // force HTML detection off for .txt
        path_str(&txt_file),
    ]);
    // They should be identical since we extracted and re-saved the same content
    assert_eq!(out.status.code(), Some(0), "HTML and re-saved text should match");
}
