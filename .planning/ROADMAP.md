# ROADMAP

> Last activity: 2026-02-23 - Completed quick task 4: 实现内容搜索功能ripgrep风格

## Blockers/Concerns

- None currently

---

## Phases

### Phase 1: Core Infrastructure
- [ ] Project setup: Cargo.toml, dependencies
- [ ] Basic CLI structure with clap
- [ ] Logging setup (tracing)
- [ ] Error handling framework

### Phase 2: File Indexing
- [ ] NTFS MFT reader for Windows
- [ ] File system walker (cross-platform fallback)
- [ ] Index storage (SQLite or custom)
- [ ] Incremental index updates

### Phase 3: Search Engine
- [ ] Fuzzy filename search
- [ ] Regex content search
- [ ] Search result ranking
- [ ] Performance optimization

### Phase 4: User Interface
- [ ] CLI interface completion
- [ ] TUI (optional)
- [ ] Result pagination
- [ ] Search history

### Phase 5: Polish & Release
- [ ] Windows executable
- [ ] Installer/package
- [ ] Documentation
- [ ] Performance tuning

---

## Quick Tasks Completed

| # | Description | Date | Commit | Status | Directory |
|---|-------------|------|--------|--------|---------------|
| 1 | Core Project Setup | 2026-02-23 | - | pending | .planning/quick/1-rust-everything-ripgrep |
