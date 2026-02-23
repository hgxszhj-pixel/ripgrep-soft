---
phase: quick-3-ntfs-mft-windows
plan: 1
type: execute
wave: 1
depends_on: []
files_modified: [Cargo.toml, src/mft_reader.rs, src/index.rs]
autonomous: true
requirements: [quick-3]
user_setup: []
---

<objective>
Add NTFS MFT (Master File Table) support for Windows fast file indexing. This enables instant filename search by reading NTFS filesystem metadata directly, similar to Everything's speed.
</objective>

<context>
@Cargo.toml
@src/index.rs

Current project state:
- Basic FileIndex structure exists with directory walking
- Need NTFS MFT parsing for instant Windows file listing
- Cargo.toml has: clap, tracing, regex, serde, thiserror, anyhow
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add NTFS MFT dependencies to Cargo.toml</name>
  <files>Cargo.toml</files>
  <action>
Add Windows-specific NTFS MFT reading crate. Use `ntfs` or similar crate for parsing NTFS structures.
- Add `ntfs` crate for MFT parsing (or `winapi`/`windows-rs` for raw API access)
- Add `windows` crate for Handle/FILE APIs if needed
- Keep dependencies minimal - focus on MFT reading only
  </action>
  <verify>
`cargo check` passes without errors related to new dependencies
  </verify>
  <done>Cargo.toml updated with NTFS-related dependencies</done>
</task>

<task type="auto">
  <name>Task 2: Create NTFS MFT reader module</name>
  <files>src/mft_reader.rs</files>
  <action>
Create new `src/mft_reader.rs` module:
- Implement `MftReader` struct for reading NTFS MFT
- Use Windows API to open volume and read $MFT (Master File Table)
- Parse MFT records to extract: filename, size, timestamps
- Handle both USN journal and traditional MFT reading
- Add error handling with thiserror for Windows API failures
- Include conditional compilation: `#[cfg(windows)]` for Windows-only code

Key implementation:
- Open NTFS volume with CreateFileW
- Read $MFT file record (record 0)
- Parse FILE_RECORD_SEGMENT_HEADER and attributes
- Extract FILE_NAME attributes (both DOS and Win32 names)
- Return Vec<FileEntry> matching existing index.rs interface
  </action>
  <verify>
Module compiles: `cargo check --lib` passes
Unit tests pass: `cargo test mft_reader`
  </verify>
  <done>MFT reader module exists with MftReader struct and read_entries() method</done>
</task>

<task type="auto">
  <name>Task 3: Integrate MFT reader with FileIndex</name>
  <files>src/index.rs, src/lib.rs</files>
  <action>
Update existing index.rs to support MFT-based indexing:
- Add `use_mft` method to FileIndex for Windows fast indexing
- Add platform detection (Windows vs cross-platform fallback)
- Export MftReader from lib.rs

Integration pattern:
```rust
impl FileIndex {
    /// Create index from NTFS MFT (Windows only)
    #[cfg(windows)]
    pub fn from_mft(volume: &str) -> Result<Self, MftError> {
        let reader = MftReader::new(volume)?;
        let entries = reader.read_entries()?;
        Ok(Self { entries })
    }
}
```

Add fallback: keep existing walk_directory for non-Windows or non-NTFS volumes
  </verify>
  <verify>
`cargo check` passes, existing tests still pass
  </verify>
  <done>FileIndex supports MFT-based indexing via from_mft() method</done>
</task>

</tasks>

<verification>
- [ ] Cargo.toml has NTFS-related dependencies
- [ ] src/mft_reader.rs exists with MFT parsing logic
- [ ] FileIndex has from_mft() method for Windows fast indexing
- [ ] Code compiles on Windows
- [ ] Existing tests pass
</verification>

<success_criteria>
NTFS MFT support enables instant file listing on Windows NTFS volumes, similar to Everything's speed. Users can call `from_mft("C:")` to build index from MFT instead of walking directories.
</success_criteria>

<output>
After completion, create `.planning/quick/3-ntfs-mft-windows/3-PLAN-SUMMARY.md`
</output>
