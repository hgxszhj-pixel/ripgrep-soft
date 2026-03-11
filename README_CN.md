# TurboSearch

高性能文件和内容搜索工具，融合了 Everything 的即时文件名搜索和 ripgrep 强大的内容匹配能力。

## 功能特性

- **快速文件索引**：使用并行 walkdir 快速索引目录
- **模糊搜索**：基于相关性评分的模糊匹配
- **通配符模式**：支持 `*.mp4`、`test*.txt`、`**/*.log` 等模式
- **正则搜索**：完整正则表达式匹配
- **内容搜索**：支持正则的文件内容搜索
- **搜索历史**：记录搜索查询历史
- **GUI 模式**：现代化的 Everything 风格图形界面
- **双击播放**：直接打开媒体文件（视频/音频）
- **文件预览**：预览文本、图片和文档信息

## 环境要求（开发）

- **Rust**：通过 [rustup.rs](https://rustup.rs/) 安装或使用 Visual Studio Build Tools
- **Windows 10/11**：推荐，用于 NTFS MFT 支持

## 快速开始

### 方式一：直接运行（推荐）

1. 克隆项目后构建：
```bash
cargo build --release
```

2. 双击 `start.bat` 启动 GUI（无控制台窗口）

或直接双击 `target\release\turbo-search.exe`

### 方式二：从命令行运行

```bash
# Debug 模式
cargo run

# Release 模式
cargo run --release

# CLI 模式
cargo run -- search --path C:\Users --pattern "document"
```

## 使用方法

### GUI 模式

双击 `start.bat` 或 `turbo-search.exe`：
- 顶部搜索栏
- 中间搜索结果
- 底部文件预览
- 双击打开文件或播放媒体

### CLI 模式

打开命令提示符或 PowerShell：

```cmd
# 按文件名搜索
turbo-search.exe search --path C:\Users --pattern "document"

# 通配符搜索
turbo-search.exe search --path D:\ --pattern "*.pdf"

# 搜索文件内容
turbo-search.exe search --path C:\Projects --content "TODO"

# 构建索引
turbo-search.exe index --path C:\Users\YourName\Documents
```

## 发布说明

构建后的文件位于 `target\release\turbo-search.exe`：

```
turbo-search/
├── target/release/
│   └── turbo-search.exe    # 主程序
├── start.bat               # GUI 启动器（双击运行）
└── README_CN.md
```

**无需安装 Rust！** exe 是独立的 Windows 可执行文件，可以直接分发给用户。

## 常见问题

### GUI 无法启动
```
cargo run
```
查看终端错误信息。

### 搜索慢
- 使用更小的搜索路径
- 先构建索引：`turbo-search.exe index --path <路径>`

## 许可证

MIT

## 作者

hgxszhj &lt;hgxszhj@gmail.com&gt;
