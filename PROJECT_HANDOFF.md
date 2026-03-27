# TurboSearch 项目接手文档

**项目**: TurboSearch (原 ripgrep-soft)
**交接日期**: 2026-03-27
**最后维护者**: Claude Code
**交接原因**: 同事继续开发

---

## 一、项目概述

TurboSearch 是一个 Windows 高性能文件搜索工具，结合了 Everything 的文件名快速搜索和 ripgrep 的内容匹配功能。支持 CLI 和 GUI 两种模式。

**技术栈**:
- Rust (2021 edition)
- eframe/egui (GUI)
- walkdir + jwalk (目录遍历)
- rayon (并行处理)

**关键文件**:
```
src/
├── main.rs           # 入口点，CLI 分发
├── cli.rs            # CLI 参数解析
├── cli_search.rs     # CLI 搜索实现
├── search.rs         # 搜索功能
├── index.rs          # 文件索引 (核心模块)
├── gui.rs            # GUI实现 (~1900行)
├── file_watcher.rs   # 文件监控
├── config.rs         # 配置管理
├── history.rs        # 搜索历史
├── utils.rs          # 工具函数
├── error.rs          # 错误类型
├── logging.rs        # 日志初始化
└── mft_reader.rs     # NTFS MFT 读取 (Windows)
```

---

## 二、当前状态

### 构建状态
```
✅ cargo build      # 通过
✅ cargo clippy     # 无警告
✅ cargo test       # 8 tests passed (集成测试)
```

### Git 状态
- 分支: main
- 领先 origin/main: 0 commits (已同步)
- 未跟踪文件: `carbonyl-proxy.bat`, `index_._src.json`

### 已完成功能
1. **CLI 搜索** - 文件名搜索、内容搜索、正则、glob
2. **GUI 搜索** - 基于 eframe/egui 的图形界面
3. **文件索引** - 多种遍历策略 (walkdir, jwalk, MFT)
4. **文件监控** - notify crate 增量更新
5. **搜索历史** - 持久化历史记录

---

## 三、最近提交 (2026-03-27)

```
7abfc67 refactor: remove dead code from search.rs and fix file_watcher warnings
8679942 refactor: remove more dead code from index.rs
7653131 refactor: remove dead code (incremental_index module, walk_directory_parallel*)
ab54825 test: add file_watcher unit tests (9 new tests)
970cccb perf: implement comprehensive performance improvements
```

---

## 四、待处理事项

### 建议

1. **GUI 拆分** - `gui.rs` (~1900行) 可以拆分为更小的模块
   - gui/components.rs - UI 组件
   - gui/state.rs - 状态管理
   - gui/handlers.rs - 事件处理

2. **DebouncedWatcher 缺失** - `file_watcher.rs` 中的测试引用了未实现的 `DebouncedWatcher`
   - 需实现或删除相关测试

3. **代码覆盖率** - 建议安装 cargo-llvm-cov 测量测试覆盖率
   ```bash
   cargo install cargo-llvm-cov
   cargo llvm-cov --html
   ```

---

## 五、开发命令

```bash
# 开发构建
cargo build

# Release 构建
cargo build --release

# 运行测试
cargo test

# Clippy 检查
cargo clippy

# 运行 CLI
./target/debug/turbo-search.exe search --path ./src --pattern "FileIndex"
./target/debug/turbo-search.exe index --path ./src

# 运行 GUI
cargo run -- --gui
```

---

## 六、配置和密钥

### GitHub Token (已撤销)
- ⚠️ 旧 Token `github_pat_11BYBMNIA0...` 已泄露并撤销
- 如需 GitHub 集成，需重新配置 Token

### 环境变量
```bash
# 可选配置
GITHUB_TOKEN=your_new_token  # GitHub API 访问
```

---

## 七、工作流技能升级

已完成的 workflow skill 升级（位于 `C:\Users\zhj\.claude\skills\workflow\`）：

| 组件 | 说明 |
|------|------|
| Phase Gate System | TDD 7 阶段 + 6 个 Gate 检查点 |
| ETHOS 设计原则 | 渐进式披露、单一职责、评审分离 |
| Conductor 配置 | Agent 编排、OWASP Top 10 安全检查 |
| Agent Protocol | 广播/顺序/回调通信模式 |
| Canary/Release | 灰度发布和正式发布工作流 |
| Feedback Loop | 反馈收集→分析→验证→固化闭环 |
| Zod 验证规则 | API 数据验证 (Zod + TypeScript) |

---

## 八、联系方式

如有问题，请查看:
- `CLAUDE.md` - 项目指南
- `src/*.rs` - 代码注释
- `C:\Users\zhj\.claude\skills\workflow\` - 工作流技能

---

*本文档由 Claude Code 自动生成*
