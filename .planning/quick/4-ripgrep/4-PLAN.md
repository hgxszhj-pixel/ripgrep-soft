---
phase: quick-04-ripgrep
plan: 4
type: execute
wave: 1
depends_on: []
files_modified:
  - src/search.rs
  - src/cli.rs
  - src/lib.rs
autonomous: true
requirements: []
---

<objective>
Add ripgrep-style content search to search file contents (not just filenames)

Purpose: Enable searching INSIDE file contents using regex patterns, matching how ripgrep works
Output: Content search with line numbers, context, and file path output
</objective>

<context>
@src/search.rs - Current search module (only searches filenames)
@src/index.rs - File index module
@src/lib.rs - Module exports
</context>

<tasks>

<task type="auto">
  <name>Add content search types and searcher</name>
  <files>src/search.rs</files>
  <action>
Add `ContentSearchQuery` struct with fields: pattern (String), case_sensitive (bool), regex (bool), max_context (usize)
Add `ContentMatch` struct with fields: file_path (PathBuf), line_number (usize), line_content (String), matched_text (String)
Add `ContentSearcher::search_files()` method that:
  - Takes query + list of paths to search
  - Opens each file, reads line by line
  - Uses regex or substring matching based on query.regex
  - Returns Vec<ContentMatch> with line numbers and content
  - Skip binary files (check for null bytes in first 8KB)
  - Use regex crate for pattern matching (already a dependency)
  </action>
  <verify>cargo test search::content</verify>
  <done>ContentSearcher can search file contents and return matches with line numbers</done>
</task>

<task type="auto">
  <name>Add content search CLI integration</name>
  <files>src/cli.rs</files>
  <action>
Add `--content` / `-c` flag to CLI for content search mode
Add `--context` / `-C` flag to specify lines of context around matches (default: 0)
When content mode is active:
  - Accept search pattern as argument
  - Search files from current directory or specified path
  - Output format: file_path:line_number:matched_line (like grep)
  - If context > 0, show surrounding lines with "-" prefix
  </action>
  <verify>cargo build && cargo run -- --content "test" --help</verify>
  <done>CLI supports content search with ripgrep-style output</done>
</task>

<task type="auto">
  <name>Add unit tests for content search</name>
  <files>src/search.rs</files>
  <action>
Add tests for:
  - ContentSearchQuery construction
  - Substring content search (case insensitive)
  - Regex content search
  - Line number tracking
  - Binary file detection
  - Empty pattern handling
  </action>
  <verify>cargo test content</verify>
  <done>All content search tests pass</done>
</task>

</tasks>

<verification>
cargo build passes without errors
cargo test passes for all search module tests
</verification>

<success_criteria>
- Content search returns matches with file path, line number, and line content
- Supports both substring and regex patterns
- CLI output format similar to grep/ripgrep: "file:line:content"
- Binary files are automatically skipped
</success_criteria>

<output>
After completion, create .planning/quick/4-ripgrep/4-SUMMARY.md
</output>
