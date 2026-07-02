//! Source-code compressor — Rust MVP, **not** a port of
//! `cassandra.transforms.code_compressor.CodeAwareCompressor`.
//!
//! # Why not a port
//!
//! Python's `CodeAwareCompressor` is tree-sitter AST parsing across
//! seven languages (Python, JavaScript, TypeScript, Go, Rust, Java,
//! C/C++) with semantic function-importance ranking (~2,100 LOC, plus
//! a `tree_sitter_language_pack` dependency it treats as optional
//! precisely because it's heavy — ~50MB of grammar artifacts). Even
//! Python's own fallback path (when tree-sitter is unavailable)
//! doesn't reimplement anything simpler — it delegates to Kompress,
//! which is *also* unwired on the Rust side (see the sibling TODO in
//! `live_zone.rs` for `ContentType::PlainText`).
//!
//! A faithful port is real, multi-day work with a new heavy
//! dependency tree. This module is a deliberately scoped MVP that
//! closes the "`SourceCode` routes to `NoOp` today" gap
//! (`REALIGNMENT/04-phase-B-live-zone.md` PR-B3) with a genuinely
//! different, much simpler algorithm: no AST, no tree-sitter, pure
//! line-based heuristics. Real value (source code currently gets
//! *zero* compression) without the dependency and time cost of the
//! full port.
//!
//! # Strategy
//!
//! 1. Below [`CodeCompressorConfig::min_lines_for_compression`],
//!    return the input unchanged — mirrors the sibling compressors'
//!    short-input bypass ([`super::log_compressor`],
//!    [`super::diff_compressor`]).
//! 2. Classify every line into one of three heuristic tiers (see
//!    [`LineKind`]). **Signature** lines (declarations / scope
//!    openers) are the load-bearing safety property: they are never
//!    touched, so a reader of the compressed output can always see
//!    every declared symbol even when a body is elided.
//! 3. Collapse runs of 2+ consecutive blank lines to exactly 1 —
//!    always safe, never changes what the code means.
//! 4. Strip comment-only lines entirely (docstrings/explanatory
//!    comments are usually not essential for an LLM to understand
//!    *what* the code does structurally).
//! 5. For each contiguous run of plain body lines longer than
//!    [`CodeCompressorConfig::max_body_run_lines`], keep the first
//!    [`CodeCompressorConfig::keep_head_lines`] and last
//!    [`CodeCompressorConfig::keep_tail_lines`] lines of the run,
//!    replacing the middle with a single omission marker.
//!
//! # What this is NOT
//!
//! - **Not syntax-validity-guaranteeing.** Python's AST-based
//!   compressor promises output that always parses; this heuristic
//!   compressor does not (a truncated body is not valid on its own,
//!   though the always-kept signature lines mean the code's public
//!   shape stays intact). The output is read by an LLM, not executed
//!   — an acceptable tradeoff for what this delivers.
//! - **Not semantic.** No function-importance ranking, no call-graph
//!   or symbol-usage awareness.
//! - **Not multi-language-tuned.** The heuristics below are chosen to
//!   work reasonably across C-like and Python-like syntax
//!   simultaneously (the two broad families covering the large
//!   majority of real tool-result code), not tuned per language the
//!   way tree-sitter grammars would be.
//!
//! CCR retrieval-marker injection (so the LLM can fetch the untouched
//! original) is handled centrally by the live-zone dispatcher's
//! `maybe_inject_ccr_marker`, exactly as it is for every sibling
//! compressor — this module does not implement its own.

/// Configuration. All fields have sane MVP defaults; nothing here
/// claims Python parity (unlike [`super::diff_compressor`]).
#[derive(Debug, Clone)]
pub struct CodeCompressorConfig {
    /// Inputs with fewer lines than this are returned unchanged.
    /// Mirrors the sibling compressors' short-input bypass, and
    /// stacks with (does not replace) the byte-threshold gate PR-B4
    /// already applies before any compressor runs.
    pub min_lines_for_compression: usize,
    /// A contiguous run of plain body lines longer than this is a
    /// truncation candidate.
    pub max_body_run_lines: usize,
    /// How many lines from the START of an over-long body run to
    /// keep verbatim.
    pub keep_head_lines: usize,
    /// How many lines from the END of an over-long body run to keep
    /// verbatim.
    pub keep_tail_lines: usize,
}

impl Default for CodeCompressorConfig {
    fn default() -> Self {
        Self {
            min_lines_for_compression: 20,
            max_body_run_lines: 8,
            keep_head_lines: 3,
            keep_tail_lines: 2,
        }
    }
}

/// Per-line heuristic classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    /// Whitespace-only.
    Blank,
    /// A declaration / scope-opening line. Never touched.
    Signature,
    /// A comment-only line (start-of-line marker, never a mid-line
    /// substring match — avoids false-positiving on strings/URLs
    /// that merely contain a comment character).
    Comment,
    /// Everything else.
    Body,
}

/// Compression result.
#[derive(Debug, Clone)]
pub struct CodeCompressionResult {
    pub compressed: String,
    pub original: String,
    pub original_line_count: usize,
    pub compressed_line_count: usize,
    pub comment_lines_dropped: usize,
    pub body_lines_truncated: usize,
}

impl CodeCompressionResult {
    pub fn was_modified(&self) -> bool {
        self.compressed != self.original
    }
}

#[derive(Debug, Clone, Default)]
pub struct CodeCompressor {
    config: CodeCompressorConfig,
}

impl CodeCompressor {
    pub fn new(config: CodeCompressorConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &CodeCompressorConfig {
        &self.config
    }

    pub fn compress(&self, content: &str) -> CodeCompressionResult {
        let lines: Vec<&str> = content.split('\n').collect();
        let original_line_count = lines.len();

        if original_line_count < self.config.min_lines_for_compression {
            return CodeCompressionResult {
                compressed: content.to_string(),
                original: content.to_string(),
                original_line_count,
                compressed_line_count: original_line_count,
                comment_lines_dropped: 0,
                body_lines_truncated: 0,
            };
        }

        let kinds: Vec<LineKind> = lines.iter().map(|l| classify_line(l)).collect();

        let mut out: Vec<String> = Vec::with_capacity(lines.len());
        let mut comment_lines_dropped = 0usize;
        let mut body_lines_truncated = 0usize;
        let mut prev_blank = false;
        let mut i = 0usize;

        while i < lines.len() {
            match kinds[i] {
                LineKind::Blank => {
                    // Collapse 2+ consecutive blank lines to exactly 1.
                    if !prev_blank {
                        out.push(String::new());
                    }
                    prev_blank = true;
                    i += 1;
                }
                LineKind::Signature => {
                    out.push(lines[i].to_string());
                    prev_blank = false;
                    i += 1;
                }
                LineKind::Comment => {
                    // Dropped entirely -- not emitted to `out`.
                    comment_lines_dropped += 1;
                    prev_blank = false;
                    i += 1;
                }
                LineKind::Body => {
                    // Find the extent of this contiguous body run.
                    let run_start = i;
                    let mut run_end = i;
                    while run_end < lines.len() && kinds[run_end] == LineKind::Body {
                        run_end += 1;
                    }
                    let run = &lines[run_start..run_end];
                    if run.len() > self.config.max_body_run_lines {
                        let head = self.config.keep_head_lines.min(run.len());
                        let tail = self.config.keep_tail_lines.min(run.len() - head);
                        let omitted = run.len() - head - tail;
                        for line in &run[..head] {
                            out.push(line.to_string());
                        }
                        if omitted > 0 {
                            out.push(format!("... ({omitted} lines omitted) ..."));
                            body_lines_truncated += omitted;
                        }
                        for line in &run[run.len() - tail..] {
                            out.push(line.to_string());
                        }
                    } else {
                        for line in run {
                            out.push(line.to_string());
                        }
                    }
                    prev_blank = false;
                    i = run_end;
                }
            }
        }

        let compressed = out.join("\n");
        let compressed_line_count = out.len();
        CodeCompressionResult {
            compressed,
            original: content.to_string(),
            original_line_count,
            compressed_line_count,
            comment_lines_dropped,
            body_lines_truncated,
        }
    }
}

fn classify_line(line: &str) -> LineKind {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return LineKind::Blank;
    }
    if is_comment_line(trimmed) {
        return LineKind::Comment;
    }
    if is_signature_line(trimmed) {
        return LineKind::Signature;
    }
    LineKind::Body
}

/// Start-of-line comment markers across common languages. Deliberately
/// anchored at the start of the trimmed line -- never a substring
/// match anywhere in the line, which would false-positive on string
/// literals or URLs containing `//` or `#`.
fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with("--")
}

/// Declaration / scope-opener keywords across common languages, plus
/// a "trimmed content ends with `{` or `:`" fallback that catches
/// scope openers the keyword list doesn't name explicitly (e.g. a
/// bare `else:`, `} else if (...) {`, or an anonymous block opener).
const SIGNATURE_KEYWORDS: &[&str] = &[
    "def ",
    "fn ",
    "pub fn ",
    "async fn ",
    "pub async fn ",
    "class ",
    "struct ",
    "pub struct ",
    "impl ",
    "pub impl ",
    "interface ",
    "func ",
    "function ",
    "public ",
    "private ",
    "protected ",
    "import ",
    "use ",
    "pub use ",
    "from ",
    "package ",
    "namespace ",
    "trait ",
    "pub trait ",
    "enum ",
    "pub enum ",
    "module ",
    "type ",
    "pub type ",
    "const ",
    "pub const ",
    "static ",
    "pub static ",
    "export ",
];

fn is_signature_line(trimmed: &str) -> bool {
    if SIGNATURE_KEYWORDS.iter().any(|kw| trimmed.starts_with(kw)) {
        return true;
    }
    if trimmed.ends_with('{') || trimmed.ends_with(':') {
        return true;
    }
    is_scope_closer(trimmed)
}

/// A standalone scope-closing line: after stripping one trailing `;`
/// or `,`, every character is a closing bracket (`}`, `)`, `]`). Very
/// common in C-like code (`}`, `});`, `)),`) and never legitimately a
/// body *statement* on its own — treating these as signature-like
/// (always kept) keeps compressed output visually bracket-balanced
/// instead of truncating away the closer of a block whose opener
/// survived.
fn is_scope_closer(trimmed: &str) -> bool {
    let core = trimmed
        .strip_suffix(';')
        .or_else(|| trimmed.strip_suffix(','))
        .unwrap_or(trimmed);
    !core.is_empty() && core.chars().all(|c| matches!(c, '}' | ')' | ']'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmp() -> CodeCompressor {
        CodeCompressor::new(CodeCompressorConfig::default())
    }

    #[test]
    fn short_input_below_threshold_returned_unchanged() {
        let c = cmp();
        let code = "def f():\n    return 1\n";
        let result = c.compress(code);
        assert_eq!(result.compressed, code);
        assert!(!result.was_modified());
    }

    #[test]
    fn collapses_multiple_blank_lines_to_one() {
        let c = CodeCompressor::new(CodeCompressorConfig {
            min_lines_for_compression: 1,
            ..Default::default()
        });
        let code = "def f():\n\n\n\n    return 1\n";
        let result = c.compress(code);
        assert!(
            !result.compressed.contains("\n\n\n"),
            "expected blank-line runs collapsed to a single blank line, got: {:?}",
            result.compressed
        );
    }

    #[test]
    fn strips_comment_only_lines() {
        let c = CodeCompressor::new(CodeCompressorConfig {
            min_lines_for_compression: 1,
            ..Default::default()
        });
        let code = "def f():\n    # this explains the next line\n    return 1\n";
        let result = c.compress(code);
        assert!(!result.compressed.contains("this explains"));
        assert_eq!(result.comment_lines_dropped, 1);
    }

    #[test]
    fn never_drops_a_line_containing_hash_mid_line() {
        // A `#` that isn't at the start of the trimmed line (e.g.
        // inside a string) must NOT be classified as a comment.
        let c = CodeCompressor::new(CodeCompressorConfig {
            min_lines_for_compression: 1,
            ..Default::default()
        });
        let code = "def f():\n    x = \"value # not a comment\"\n    return x\n";
        let result = c.compress(code);
        assert!(result.compressed.contains("value # not a comment"));
    }

    #[test]
    fn signature_lines_always_survive() {
        let c = CodeCompressor::new(CodeCompressorConfig {
            min_lines_for_compression: 1,
            max_body_run_lines: 2,
            keep_head_lines: 1,
            keep_tail_lines: 1,
        });
        let mut code = String::from("def big_function():\n");
        for i in 0..30 {
            code.push_str(&format!("    x{i} = {i}\n"));
        }
        code.push_str("    return x0\n");
        let result = c.compress(&code);
        assert!(
            result.compressed.starts_with("def big_function():"),
            "signature must survive verbatim at the top: {:?}",
            result.compressed
        );
        assert!(result.was_modified());
        assert!(result.body_lines_truncated > 0);
    }

    #[test]
    fn truncates_long_body_run_keeping_head_and_tail() {
        let c = CodeCompressor::new(CodeCompressorConfig {
            min_lines_for_compression: 1,
            max_body_run_lines: 5,
            keep_head_lines: 2,
            keep_tail_lines: 2,
        });
        let mut code = String::from("fn f() {\n");
        for i in 0..20 {
            code.push_str(&format!("    line_{i};\n"));
        }
        code.push_str("}\n");
        let result = c.compress(&code);
        assert!(result.compressed.contains("line_0;"));
        assert!(result.compressed.contains("line_1;"));
        assert!(result.compressed.contains("line_18;"));
        assert!(result.compressed.contains("line_19;"));
        assert!(result.compressed.contains("lines omitted"));
        // Middle lines must be gone.
        assert!(!result.compressed.contains("line_10;"));
        // The closing brace must survive verbatim as its own
        // signature-like line, not get swept into the truncated body
        // run and silently dropped or double-counted as the "tail".
        assert!(result.compressed.trim_end().ends_with('}'));
    }

    #[test]
    fn standalone_closing_brace_is_never_truncated_away() {
        assert!(is_signature_line("}"));
        assert!(is_signature_line("});"));
        assert!(is_signature_line("]),"));
        assert!(is_signature_line(")))"));
        // A real body statement that merely CONTAINS closing brackets
        // must not be misclassified.
        assert!(!is_signature_line("foo(bar[0]);"));
        assert!(!is_signature_line("x = compute(y);"));
    }

    #[test]
    fn short_body_run_not_truncated() {
        let c = CodeCompressor::new(CodeCompressorConfig {
            min_lines_for_compression: 1,
            max_body_run_lines: 8,
            ..Default::default()
        });
        let code = "fn f() {\n    a();\n    b();\n    c();\n}\n";
        let result = c.compress(code);
        assert!(result.compressed.contains("a();"));
        assert!(result.compressed.contains("b();"));
        assert!(result.compressed.contains("c();"));
        assert_eq!(result.body_lines_truncated, 0);
    }

    #[test]
    fn empty_input_does_not_panic() {
        let c = CodeCompressor::new(CodeCompressorConfig {
            min_lines_for_compression: 0,
            ..Default::default()
        });
        let result = c.compress("");
        assert_eq!(result.compressed, "");
    }

    #[test]
    fn python_style_signature_detection() {
        assert!(is_signature_line("def foo(x, y):"));
        assert!(is_signature_line("class Foo:"));
        assert!(is_signature_line("if x:"));
        assert!(is_signature_line("import os"));
        assert!(is_signature_line("from foo import bar"));
        assert!(!is_signature_line("return x"));
        assert!(!is_signature_line("x = compute(y)"));
    }

    #[test]
    fn c_like_signature_detection() {
        assert!(is_signature_line("public class Foo {"));
        assert!(is_signature_line("fn f() {"));
        assert!(is_signature_line("struct Point {"));
        assert!(is_signature_line("} else {"));
        assert!(!is_signature_line("let x = 1;"));
    }

    #[test]
    fn comment_style_detection() {
        assert!(is_comment_line("// a comment"));
        assert!(is_comment_line("# a comment"));
        assert!(is_comment_line("* continuation of a block comment"));
        assert!(is_comment_line("/* start of block comment"));
        assert!(!is_comment_line(
            "x = 1 // not at line start... wait yes it's not"
        ));
    }

    /// Byte-safety property: for any input, `compress` must never
    /// panic and must always produce valid UTF-8 (guaranteed by
    /// construction since we only slice on `\n` and never touch
    /// multi-byte boundaries mid-codepoint, but this test exercises
    /// the property directly against adversarial-shaped input).
    #[test]
    fn does_not_panic_on_unusual_input() {
        let c = cmp();
        let many_emoji = "🎉🎊 def f(): 🎉🎊\n".repeat(10);
        let long_line = "a".repeat(100_000);
        let inputs: [&str; 4] = [
            "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n",
            "###########################################\n",
            many_emoji.as_str(),
            long_line.as_str(),
        ];
        for input in inputs {
            let _ = c.compress(input);
        }
    }
}
