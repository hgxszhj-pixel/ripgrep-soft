# 收藏夹功能 - 实现计划

## 功能概述
允许用户保存常用的搜索条件为收藏夹，快速调用进行搜索。

## 功能需求

### 1. 收藏夹数据结构
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FavoriteSearch {
    pub id: String,
    pub name: String,              // 收藏夹名称
    pub search_pattern: String,     // 搜索关键词
    pub search_path: String,        // 搜索路径
    pub search_mode: SearchMode,    // 搜索模式 (Filename/Content)
    pub use_regex: bool,           // 是否使用正则
    pub use_glob: bool,             // 是否使用Glob
    pub case_sensitive: bool,       // 是否大小写敏感
    pub size_filter: String,        // 大小过滤
    pub created_at: u64,            // 创建时间
}
```

### 2. 功能点
- 添加当前搜索为收藏夹
- 查看收藏夹列表
- 从收藏夹快速搜索
- 删除收藏夹
- 编辑收藏夹名称

### 3. UI 设计
在搜索栏旁边添加收藏夹按钮：
```
[Search...] [Search] [★ Favorites ▼]
```

收藏夹下拉菜单：
```
★ Favorites
─────────────
📁 Documents (*.pdf, *.doc)
📁 Projects (src, test)
📁 Media (*.mp4, *.mp3)
─────────────
+ Add Current Search
```

## 实现步骤

| 步骤 | 任务 | 文件 |
|------|------|------|
| 1 | 添加 FavoriteSearch 结构体 | gui/state.rs |
| 2 | 添加收藏数据管理 | gui.rs (FavoritesManager) |
| 3 | 创建收藏夹按钮和菜单 | gui.rs |
| 4 | 实现保存/加载收藏夹 | gui.rs |
| 5 | 测试验证 | - |
