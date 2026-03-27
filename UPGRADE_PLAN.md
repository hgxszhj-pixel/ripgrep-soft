# TurboSearch 项目升级计划

## 一、问题汇总

### 1.1 关键问题优先级

| 优先级 | 模块 | 问题 | 影响 |
|--------|------|------|------|
| **CRITICAL** | security | 硬编码GitHub Token | 安全风险 |
| **CRITICAL** | gui.rs | 每次文件变化全量clone FileIndex | O(n)复杂度，100K文件=100K克隆 |
| **HIGH** | search.rs | SkimMatcherV2每次搜索重建 | CPU浪费 |
| **HIGH** | file_watcher.rs | 无事件debouncing | 批量操作时线程爆炸 |
| **HIGH** | index.rs | 无并行目录遍历 | 多核CPU利用率低 |
| **HIGH** | 代码质量 | 测试覆盖率低 | 质量风险 |

### 1.2 模块化升级顺序

```
阶段1: 安全修复 (必须先做)
  └── 移除硬编码Token

阶段2: 性能优化 - GUI层
  ├── 2.1 修复FileIndex增量更新 (替换全量clone)
  └── 2.2 预览缓存优化

阶段3: 性能优化 - 搜索层
  ├── 3.1 SkimMatcherV2缓存
  ├── 3.2 搜索结果排序修复
  └── 3.3 正则缓存并发安全

阶段4: 性能优化 - 索引层 (与阶段2/3并行)
  ├── 4.1 整合并行目录遍历 (统一使用jwalk)
  └── 4.2 MFT Reader重构

阶段5: 架构重构 (依赖阶段2/3/4完成)
  ├── 5.1 消除循环依赖
  ├── 5.2 统一错误处理
  └── 5.3 代码去重

阶段6: 测试补充
  └── 6.1 单元测试覆盖
```

---

## 二、详细升级任务

### 阶段1: 安全修复 (必须先做)

#### T1.1: 移除硬编码GitHub Token
- **文件**: `src/heartbeat/config.rs`
- **问题**: 硬编码GitHub PAT，已泄露
- **操作**:
  1. 使用环境变量 `GITHUB_TOKEN` 替代硬编码
  2. **【新】** 添加 `#[serde(skip)]` 防止Token序列化到配置文件
  3. **【新】** 添加启动验证：Token格式检查
  4. **【新】** 实现无Token时的graceful fallback
- **额外安全修复**:
  - **【新】** mft_reader.rs: 修复unsafe块中的unwrap() - 使用match替代
  - **【新】** fetcher.rs: 添加请求间隔(100ms)防止API限流
  - **【新】** HTTP客户端: 使用rustls_tls
- **验证**:
  1. `grep -r "github_pat" src/` 无结果
  2. `grep -r "GITHUB_TOKEN" .env*` 无结果 (或.gitignore包含)
  3. 序列化测试确认token字段被跳过
  4. `cargo clippy` 无警告
  5. `cargo audit` 无高危漏洞

#### T1.2: 配置Token持久化风险修复
- **文件**: `src/heartbeat/config.rs`
- **问题**: `FetcherConfig` 实现Serialize，Token可能泄露到配置文件
- **操作**: 添加 `#[serde(skip)]` 到token字段

---

### 阶段2: GUI层性能优化

#### T2.1: FileIndex增量更新 (CRITICAL)
- **文件**: `src/gui.rs`, `src/incremental_index.rs`
- **问题**: `let mut index = (*self.index).clone()` 每次全量克隆 - O(n)复杂度
- **根因**: `add_entry()`/`remove_entry()` 增量方法已存在，但被全量clone抵消
- **解决方案**:
  - **【修正】** 使用 `Rc<RefCell<FileIndex>>` 替代 `Arc<FileIndex>`
  - GUI层直接调用 `index.borrow_mut().add_entry()` 而非clone+替换
  - **注意**: egui单线程模型天然适合RefCell的内部可变性
- **同步修复**: `incremental_index.rs:133` 也有同样问题，一并修复
- **目标**: O(1) 每次文件变化更新
- **验证**: 大目录(100K+文件)下文件变化响应时间 < 100ms

#### T2.2: 预览缓存优化
- **文件**: `src/gui.rs`
- **问题**: 预览缓存存储完整文件内容(最高50MB+)
- **解决方案**: 只缓存前N行/最后N行，限制总大小
- **验证**: 内存占用降低50%+

#### T2.3: 文件监控debouncing
- **文件**: `src/file_watcher.rs`
- **问题**: 无debouncing，批量操作生成大量事件
- **解决方案**: 添加100ms窗口内事件合并
- **验证**: 批量文件操作事件数量减少90%+

---

### 阶段3: 搜索层性能优化

#### T3.1: SkimMatcherV2缓存
- **文件**: `src/search.rs`
- **问题**: `fuzzy_search_owned` 每次创建新实例
- **解决方案**: 使用 `LazyLock` 或 `OnceLock` 缓存实例
- **验证**: 连续搜索性能提升20%+

#### T3.2: 搜索结果排序修复
- **文件**: `src/search.rs`
- **问题**: `select_nth_unstable_by` + `sort_by` 逻辑不清晰
- **解决方案**: 使用 `select_n` 然后截取
- **验证**: 搜索结果top-N正确性测试通过

#### T3.3: 正则缓存并发安全
- **文件**: `src/search.rs`
- **问题**: check-then-remove非原子操作
- **解决方案**: 使用 `RwLock` 保护，或者改用 `DashMap`
- **验证**: 并发压力测试通过

---

### 阶段4: 索引层性能优化

#### T4.1: 整合并行目录遍历
- **文件**: `src/index.rs`
- **问题**: 并行遍历已存在(jwalk, rayon)，但不一致
  - `walk_directory_jwalk`: 使用jwalk，已并行
  - `walk_directory_recursive`: 使用par_iter，已并行
  - `walk_directory_limited`: 单线程WalkDir
- **解决方案**: 统一所有遍历路径使用jwalk，确保一致并行
- **目标**: 多核CPU利用率达到80%+
- **验证**: 索引100K文件时间减少40%+

#### T4.2: MFT Reader重构
- **文件**: `src/mft_reader.rs`
- **问题**: 未真正读取MFT，只是用FindFirstFile
- **决策**: 要么实现真正MFT读取，要么标记为deprecated并优化FindFirstFile路径
- **验证**: 如保留，优化后的遍历性能提升

---

### 阶段5: 架构重构

#### T5.1: 消除循环依赖
- **文件**: `src/config.rs`, `src/gui/state.rs`
- **问题**: config → gui::state, gui::state → config
- **解决方案**: 移动 `AppTheme`, `Favorites` 到共享模块
- **验证**: `cargo check` 通过

#### T5.2: 统一错误处理
- **文件**: `error.rs`, `config.rs`, `mft_reader.rs`
- **问题**: 4种不同错误枚举
- **解决方案**: 使用 `thiserror::Error` 的 `#[from]` 特性统一
- **验证**: 所有错误类型实现 `std::error::Error`

#### T5.3: 代码去重
- **合并**:
  - `BINARY_EXTS` 合并到 `utils.rs`
  - `SizeFilter::from_string` 合并到 `utils.rs`
  - UUID生成函数统一使用 `uuid` crate
- **验证**: 无重复代码，构建成功

---

### 阶段6: 测试补充

#### T6.1: 单元测试覆盖
- **目标模块**:
  - [ ] `index.rs` - 索引构建和查询
  - [ ] `search.rs` - 搜索和过滤
  - [ ] `file_watcher.rs` - 事件处理
  - [ ] `error.rs` - 错误类型
- **覆盖率目标**: 核心模块 80%+
- **验证**: `cargo test -- --nocapture` 全部通过

---

## 三、验证策略

每个模块升级后执行:
```bash
# 1. 构建验证
cargo build 2>&1 | rtk err

# 2. 编译警告检查
cargo clippy 2>&1 | rtk err

# 3. 测试验证
cargo test

# 4. 内存检查 (可选)
cargo build --release && valgrind --leak-check=full target/release/turbo-search.exe
```

---

## 四、风险控制

1. **回滚准备**: 每次commit前确保上一状态可用
2. **增量升级**: 每阶段验证后才进入下一阶段
3. **测试驱动**: 修复前先写测试用例
4. **最小改动**: 优先小步快跑，避免大规模重构
