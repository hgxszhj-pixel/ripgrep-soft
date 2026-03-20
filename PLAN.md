# TurboSearch 功能实现计划

## 1. 搜索结果分页功能

### 功能概述
为 GUI 搜索结果添加分页功能，解决大数据集（>5000条）一次性加载导致的性能问题和UI卡顿。

### 当前问题
- 搜索 5000+ 结果时一次性渲染所有项导致 UI 卡顿
- 滚动浏览大量结果时不流畅
- 用户难以定位特定页的结果

### 解决方案

### 1. 数据结构修改
在 `gui/state.rs` 中添加分页状态:

```rust
#[derive(Clone, Debug)]
pub struct PaginationState {
    pub current_page: usize,
    pub items_per_page: usize,
    pub total_items: usize,
}
```

### 2. GUI 修改 (gui.rs)
- 添加分页状态字段
- 实现分页 UI 组件：首页/末页/上一页/下一页按钮、页码显示、每页条数选择器、手动跳转

### 验收标准

1. ✅ 搜索结果 > 100 条时自动启用分页
2. ✅ 可以点击页码切换页面
3. ✅ 支持上一页/下一页/首页/末页
4. ✅ 支持手动输入页码跳转
5. ✅ 可以配置每页显示条数
6. ✅ 状态栏显示当前页码和总页数
7. ✅ 切换页面时保持搜索条件和选中状态

---

## 2. 收藏夹功能

### 功能概述
允许用户保存常用的搜索条件为收藏夹，快速调用进行搜索。

### 功能需求

### 1. 收藏夹数据结构
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FavoriteSearch {
    pub id: String,
    pub name: String,
    pub search_pattern: String,
    pub search_path: String,
    pub search_mode: SearchMode,
    pub use_regex: bool,
    pub use_glob: bool,
    pub case_sensitive: bool,
    pub size_filter: String,
    pub created_at: u64,
}
```

### 2. 功能点
- 添加当前搜索为收藏夹
- 查看收藏夹列表
- 从收藏夹快速搜索
- 删除收藏夹
- 编辑收藏夹名称

### 3. UI 设计
- 收藏夹按钮在搜索栏旁边
- 点击可保存当前搜索条件
- 点击收藏夹项可快速执行搜索
- 收藏夹数据保存在 `favorites.json`
