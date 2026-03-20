# 搜索结果分页功能 - 实现计划

## 功能概述
为 GUI 搜索结果添加分页功能，解决大数据集（>5000条）一次性加载导致的性能问题和UI卡顿。

## 当前问题
- 搜索 5000+ 结果时一次性渲染所有项导致 UI 卡顿
- 滚动浏览大量结果时不流畅
- 用户难以定位特定页的结果

## 解决方案

### 1. 数据结构修改

在 `gui/state.rs` 中添加分页状态:

```rust
#[derive(Clone, Debug)]
pub struct PaginationState {
    pub current_page: usize,      // 当前页码 (1-based)
    pub items_per_page: usize,    // 每页条数 (默认100)
    pub total_items: usize,       // 总结果数
}

impl PaginationState {
    pub fn total_pages(&self) -> usize {
        (self.total_items + self.items_per_page - 1) / self.items_per_page
    }

    pub fn offset(&self) -> usize {
        (self.current_page - 1) * self.items_per_page
    }

    pub fn limit(&self) -> usize {
        self.items_per_page
    }
}
```

### 2. GUI 修改 (gui.rs)

#### 2.1 添加分页状态字段
```rust
struct RipgrepApp {
    // ... existing fields
    pagination: PaginationState,
    // ...
}
```

#### 2.2 分页 UI 组件
位置: 搜索结果列表下方

```
[上一页] [1] [2] [3] ... [50] [下一页]  |  第 1/50 页  |  每页: [100 ▼]
```

组件要求:
- 首页/末页按钮
- 上一页/下一页按钮
- 页码按钮 (显示相邻5页 + 首尾页)
- 手动页码输入跳转
- 每页条数选择器 (50/100/200/500)

#### 2.3 搜索逻辑修改
- 搜索时不加载全部结果，改为按需加载
- 或一次性加载但只渲染当前页 (简单方案)

### 3. 页面范围计算

```rust
fn get_visible_pages(current: usize, total: usize) -> Vec<usize> {
    // 显示逻辑: 当前页居中，首尾页始终显示
    // 例如: 当前5页, 总50页 -> [1, 2, 3, 4, 5, ..., 50]
}
```

### 4. 配置文件扩展

在 settings.json 中添加:
```json
{
    "items_per_page": 100,
    "pagination_style": "numbered" // 或 "scroll"
}
```

## 实现步骤

| 步骤 | 任务 | 文件 | 预计行数 |
|------|------|------|----------|
| 1 | 添加 PaginationState 结构体 | gui/state.rs | +30行 |
| 2 | 在 RipgrepApp 添加分页字段 | gui.rs | +10行 |
| 3 | 实现分页计算逻辑 | gui.rs | +20行 |
| 4 | 创建分页 UI 组件 | gui.rs | +80行 |
| 5 | 集成搜索结果与分页 | gui.rs | +30行 |
| 6 | 添加每页条数设置 | gui/state.rs + gui.rs | +20行 |
| 7 | 测试验证 | - | - |

## 验收标准

1. ✅ 搜索结果 > 100 条时自动启用分页
2. ✅ 可以点击页码切换页面
3. ✅ 支持上一页/下一页/首页/末页
4. ✅ 支持手动输入页码跳转
5. ✅ 可以配置每页显示条数
6. ✅ 状态栏显示当前页码和总页数
7. ✅ 切换页面时保持搜索条件和选中状态

## 性能考虑

- 分页只是视图层优化，搜索仍返回全部结果
- 后续可优化为后端分页 (lazy loading)
- 当前方案: 一次搜索，多次分页浏览
