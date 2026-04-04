```markdown
# ripgrep-soft Development Patterns

> Auto-generated skill from repository analysis

## Overview

This skill teaches you the core development patterns, coding conventions, and common workflows used in the `ripgrep-soft` Rust codebase. You'll learn how to contribute features, maintain code quality, update documentation, and keep the repository clean, all while following established conventions and leveraging suggested commands for efficiency.

## Coding Conventions

- **File Naming:**  
  Use camelCase for file names.  
  _Example:_  
  ```
  src/fileSearch.rs
  src/guiState.rs
  ```

- **Import Style:**  
  Use relative imports within the codebase.  
  _Example:_  
  ```rust
  use crate::gui::state::GuiState;
  use super::searchUtils;
  ```

- **Export Style:**  
  Use named exports for modules and functions.  
  _Example:_  
  ```rust
  pub mod gui;
  pub fn search_files(...) { ... }
  ```

- **Commit Messages:**  
  Follow [Conventional Commits](https://www.conventionalcommits.org/) with prefixes like `docs`, `feat`, `chore`, `refactor`, `perf`, `test`.  
  _Example:_  
  ```
  feat: add fuzzy search capability to core engine
  refactor: remove unused NTFS module
  ```

## Workflows

### Feature Development with Plan and Summary
**Trigger:** When adding a significant feature (e.g., fuzzy search, NTFS MFT support) and documenting its plan and summary  
**Command:** `/new-feature-with-plan`

1. Create or update a plan file in `.planning/quick/{n-feature}/n-PLAN.md`
2. Create or update a summary file in `.planning/quick/{n-feature}/n-SUMMARY.md`
3. Update `.planning/STATE.md` and/or `.planning/ROADMAP.md` to reflect progress
4. Implement the feature in relevant `src/` files
5. Commit all related files together

_Example:_
```
docs: add plan and summary for fuzzy search
feat: implement fuzzy search in src/searchEngine.rs
```

---

### GUI Feature Addition
**Trigger:** When adding a new GUI capability (e.g., pagination, favorites)  
**Command:** `/add-gui-feature`

1. Update or create the implementation in `src/gui.rs`
2. Update or create state management in `src/gui/state.rs`
3. Optionally update `PLAN.md` with feature details
4. Commit all related files together

_Example:_
```rust
// src/gui.rs
pub fn add_pagination(...) { ... }

// src/gui/state.rs
pub struct PaginationState { ... }
```

---

### Codebase Refactor and Dead Code Removal
**Trigger:** When cleaning up the codebase by removing unused code or modules and fixing warnings  
**Command:** `/refactor-dead-code`

1. Identify dead code or unused modules in `src/`
2. Remove or refactor code in affected `.rs` files
3. Optionally update related plan or documentation files
4. Commit all related files together

_Example:_
```
refactor: remove deprecated search module
```

---

### Documentation Update for New Features
**Trigger:** When documenting new features or changes for users  
**Command:** `/update-docs`

1. Edit `README.md` and/or `README_CN.md` to describe new features
2. Commit documentation changes

_Example:_
```
docs: update README with NTFS support instructions
```

---

### Project Cleanup: Remove Planning or Build Artifacts
**Trigger:** When cleaning up the repository by removing internal planning files, docs, or build artifacts  
**Command:** `/cleanup-project`

1. Remove files/directories from git tracking (e.g., `.planning/`, `docs/`, `target/`)
2. Update `.gitignore` to exclude these paths
3. Commit the cleanup

_Example:_
```
chore: remove old planning files and update .gitignore
```

## Testing Patterns

- **Test File Pattern:**  
  Test files follow the `*.test.*` pattern (e.g., `searchEngine.test.rs`).
- **Framework:**  
  The specific test framework is not detected, but standard Rust testing practices likely apply.
- **Example Test:**  
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_fuzzy_search() {
          assert!(fuzzy_search("foo", "foobar"));
      }
  }
  ```

## Commands

| Command                 | Purpose                                                      |
|-------------------------|--------------------------------------------------------------|
| /new-feature-with-plan  | Start a new feature with planning and summary documentation  |
| /add-gui-feature        | Add a new GUI feature and update state management            |
| /refactor-dead-code     | Remove dead code or unused modules and clean up warnings     |
| /update-docs            | Update user-facing documentation for new features            |
| /cleanup-project        | Remove planning/build artifacts and update .gitignore        |
```