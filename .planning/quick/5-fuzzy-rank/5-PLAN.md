---
phase: quick
plan: 5
type: execute
wave: 1
depends_on: []
files_modified: [Cargo.toml, src/search.rs]
autonomous: true
requirements: [FUZZY-01]
user_setup: []

must_haves:
  truths:
    - "Search results are sorted by relevance score (best match first)"
    - "Fuzzy matching finds files with characters in sequence even with gaps"
    - "Higher scores indicate better matches"
  artifacts:
    - path: "src/search.rs"
      provides: "Fuzzy search with scoring and ranking"
      contains: "fuzzy_score, sorted_by_score"
---

<objective>
Add fuzzy search with result ranking/sorting functionality

Purpose: Replace simple substring matching with true fuzzy matching that scores results by relevance
Output: Ranked search results sorted by fuzzy match score
</objective>

<context>
@src/search.rs (current search implementation - uses simple contains())
@src/index.rs (FileIndex and FileEntry structures)
@Cargo.toml (dependencies)
</context>

<tasks>

<task type="auto">
  <name>Add fuzzy-matcher dependency and create scoring utility</name>
  <files>Cargo.toml</files>
  <action>
    Add fuzzy-matcher crate to Cargo.toml dependencies:
    - Add `fuzzy-matcher = "0.3"` to [dependencies]
  </action>
  <verify>cargo check passes</verify>
  <done>fuzzy-matcher crate available in project</done>
</task>

<task type="auto">
  <name>Implement fuzzy search with scoring and ranking</name>
  <files>src/search.rs</files>
  <action>
    Modify Searcher to use fuzzy matching with score-based ranking:
    
    1. Import fuzzy_matcher::FuzzyMatcher and fuzzy_matcher::skim::SkimMatcherV2
    2. Modify fuzzy_search() to:
       - Use SkimMatcherV2 for fuzzy matching
       - Get optional score for each entry using fuzzy_match()
       - Filter only matches (score > 0)
       - Sort results by score descending
    3. Return Vec<&'a FileEntry> sorted by relevance
    4. Update existing tests to account for ordering or add new ranking test
    
    Keep backward compatibility: case_sensitive option still works via FuzzyMatcher
  </action>
  <verify>cargo test passes</verify>
  <done>
    - "test.txt" with query "txt" returns results sorted by score
    - "document_test_final.rs" with query "dtf" gets higher score than "other.txt"
    - All existing tests still pass
  </done>
</task>

</tasks>

<verification>
- Run `cargo test` - all tests pass
- Verify fuzzy ranking works: search for partial patterns and check order
</verification>

<success_criteria>
- Fuzzy search now uses proper character-sequence matching
- Results are sorted by relevance score (best match first)
- All existing functionality preserved (case_sensitive, regex modes)
</success_criteria>

<output>
After completion, create `.planning/quick/5-fuzzy-rank/5-SUMMARY.md`
</output>
