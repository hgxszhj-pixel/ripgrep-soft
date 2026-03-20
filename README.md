# TurboSearch

A high-performance file and content search tool for Windows, combining Everything's instant filename search with ripgrep's powerful content matching capabilities.

## Features

- **Fast File Indexing**: Index directories quickly with parallel walkdir traversal
- **Fuzzy Search**: Find files using fuzzy matching with relevance scoring
- **Glob Patterns**: Support wildcard patterns like `*.mp4`, `test*.txt`, `**/*.log`
- **Regex Search**: Full regex pattern matching for advanced queries
- **Content Search**: Search inside files with regex support
- **Search History**: Keep track of your search queries
- **GUI Mode**: Modern Everything-style graphical interface
- **Double-click to Play**: Open media files (video/audio) directly
- **File Preview**: Preview text, images, and documents
- **Auto-detect Media Players**: Automatically detects installed video players (VLC, PotPlayer, MPC-HC, MPV, etc.)
- **Customizable Player**: Select your preferred media player in Settings
- **Animated Icons**: Visual feedback with animated status icons (lightning bolt, spinning globe, rotating arrows)
- **No Console Flash**: Clean startup without console window flicker

## Requirements

- **Rust**: Install via [rustup.rs](https://rustup.rs/) or Visual Studio Build Tools
- **Windows 10/11**: Recommended for NTFS MFT support

## Quick Start

### 1. Install Rust (if not installed)

Download and run: https://rustup.rs/

Or install Visual Studio Build Tools with C++ workload.

### 2. Clone and Build

```bash
# Clone the project
git clone https://github.com/hgxszhj/turbo-search.git
cd turbo-search

# Build
cargo build --release
```

### 3. Run

```bash
# GUI mode
./target/release/turbo-search.exe

# Or run in debug mode
cargo run
```

## Windows Usage

### GUI Mode (Recommended)

Double-click `turbo-search.exe` or run from Command Prompt:

```cmd
turbo-search.exe
```

GUI features:
- Search bar at top
- Results in the middle
- File preview at bottom
- Double-click to open files or play media
- Animated status icons showing search/indexing progress
- Settings panel for configuring media player preference
- Auto-detection of installed video players (VLC, PotPlayer, MPC-HC, MPV, etc.)

### CLI Mode

Open Command Prompt or PowerShell:

```cmd
# Search files by name
turbo-search.exe search --path C:\Users --pattern "document"

# Search with glob pattern
turbo-search.exe search --path D:\ --pattern "*.pdf"

# Search file contents
turbo-search.exe search --path C:\Projects --content "TODO"

# Use regex
turbo-search.exe search --path . --pattern "\.rs$" --regex

# Case sensitive search
turbo-search.exe search --path . --pattern "README" --case-sensitive

# Build index
turbo-search.exe index --path C:\Users\YourName\Documents

# View search history
turbo-search.exe history
```

### CLI Options

#### search
- `-p, --path`: Path to search in (default: ".")
- `--pattern`: Search pattern for filename search
- `-c, --content`: Content search pattern
- `-e, --regex`: Use regex for pattern matching
- `-i, --case-sensitive`: Case sensitive search
- `-C, --context`: Number of lines of context around matches
- `-l, --limit`: Maximum number of results (default: 100)

#### index
- `-p, --path`: Path to index (default: ".")
- `--rebuild`: Force rebuild index

#### history
- `-n, --count`: Number of recent searches (default: 10)
- `-c, --clear`: Clear search history

## Keyboard Shortcuts (GUI)

- **Enter**: Start search
- **Arrow Keys**: Navigate results
- **Double-click**: Open file / Play media
- **Ctrl+C**: Copy selected path
- **Ctrl+,** or **Settings button**: Open Settings panel

## File Support

| Type | Preview | Double-click Action |
|------|---------|-------------------|
| Text (.txt, .md, .rs, etc.) | ✅ | Open with default editor |
| Images (.png, .jpg, .gif) | ✅ | Open with default app |
| Video (.mp4, .avi, .mkv) | ❌ | Play with selected player |
| Audio (.mp3, .wav) | ❌ | Play with selected player |
| Documents (.pdf, .docx) | Info only | Open with default app |

## Supported Media Players

TurboSearch automatically detects the following video players:
- VLC
- PotPlayer (64-bit & 32-bit)
- MPC-HC (64-bit & 32-bit)
- MPV
- SMPlayer
- KMPlayer
- Windows Media Player
- GOM Player

You can select your preferred player in Settings.

## Troubleshooting

### Build Errors

**Missing Visual Studio Build Tools**:
```
error: linking with `link.exe` failed
```
Install Visual Studio Build Tools with C++ workload.

**Missing Rust**:
```
cargo: command not found
```
Install from https://rustup.rs/

### Runtime Errors

**GUI doesn't start**:
```cmd
# Try running from terminal to see error messages
cargo run
```

**Search is slow**:
- Use smaller search paths
- Build an index first: `turbo-search.exe index --path <path>`

## License

MIT

## Author

hgxszhj &lt;hgxszhj@gmail.com&gt;
# turbo-search
# ripgrep-soft
