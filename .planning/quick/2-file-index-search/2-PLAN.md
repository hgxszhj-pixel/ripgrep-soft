---
phase: quick-2
plan: 01
type: execute
wave: 1
depends_on: []
files_modified: [src/lib.rs, src/index.rs, src/search.rs]
autonomous: true
requirements: []
---

# File Index and Search Implementation

## Context

Project ripgrep-soft is a high-performance file/content search tool. The CLI structure already exists with `Search` and `Index` commands. This task implements the core file indexing and search functionality.

**Current State:**
- CLI with `Search` and `Index` subcommands exists (src/cli.rs)
- Basic logging and error handling in place
- Need to implement: file walker, index structure, and search logic

## Task Dependency Graph

| Task | Depends On | Reason |
|------|------------|--------|
| Task 1: Create index module with file walker | None | Foundation for search functionality |
| Task 2: Implement search logic | Task 1 | Requires index structure to search |

## Parallel Execution Graph

Wave 1 (Start Immediately):
- Task 1: Create index module with file walker
- Task 2: Implement search logic (depends on Task 1)

Critical Path: Task 1 → Task 2

## Tasks

### Task 1: Create File Index Module

**Description**: Create the index module with file system walker and basic index structure.

**Delegation Recommendation**:
- Category: `deep` - Requires implementing domain-specific structures
- Skills: [] - Standard Rust patterns sufficient

**Skills Evaluation**:
- ✅ Standard Rust - No specialized skills needed
- ❌ cargo-log-parser - Not needed for new implementation

**Depends On**: None

**Files**:
- src/index.rs (new file)
- src/lib.rs (add module)

**Action**:
1. Create `src/index.rs` with:
   - `FileEntry` struct: path (PathBuf), name (String), size (u64), modified (SystemTime)
   - `FileIndex` struct: Vec<FileEntry> with methods:
     - `new()` - create empty index
     - `walk_directory(path: &Path)` - traverse directory recursively
     - `add_entry(entry: FileEntry)` - add file to index
     - `len()` - return entry count

2. Update `src/lib.rs` to add: `pub mod index;`

**Verify**: `cargo build` succeeds without errors

**Acceptance Criteria**: 
- FileIndex can walk a directory and collect file entries
- Index stores path, filename, size, and modification time

---

### Task 2: Implement Search Logic

**Description**: Implement filename search functionality that integrates with the CLI.

**Delegation Recommendation**:
- Category: `deep` - Requires pattern matching logic
- Skills: [] - Standard Rust sufficient

**Depends On**: Task 1

**Files**:
- src/search.rs (new file)
- src/lib.rs (add module)

**Action**:
1. Create `src/search.rs` with:
   - `SearchQuery` struct: pattern (String), case_sensitive (bool), regex (bool)
   - `Searcher` struct with `search(query: &SearchQuery, index: &FileIndex) -> Vec<&FileEntry>` method
   - Implement exact match and fuzzy match (substring) search
   - If regex flag is enabled, use `regex` crate for pattern matching

2. Update `src/index.rs` to add `SearchQuery` import and integrate search

3. Update `src/lib.rs` to add: `pub mod search;`

4. Add regex dependency to Cargo.toml: `regex = "1.10"`

**Verify**: 
- `cargo build` succeeds
- Test with: `cargo run -- search --pattern "test"` on a sample directory

**Acceptance Criteria**:
- Search returns matching file entries
- Supports case-insensitive substring matching by default
- Optional regex search when --regex flag is used

---

## Commit Strategy

Single commit after both tasks complete: `feat(quick-2): implement file index and search`

## Success Criteria

1. `cargo build` completes without errors
2. FileIndex can walk directories and build file list
3. Search command returns matching files from the index
4. Basic integration with existing CLI structure works
