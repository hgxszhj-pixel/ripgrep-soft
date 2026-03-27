# Workflow Skill 优化计划

## 问题分析

### 当前状态
- 38个文件，总计约5121行代码/文档
- SKILL.md 主文件70行，包含所有核心规则
- 核心规则文件：rules.md (104行)
- TDD工作流：tasks/tdd/workflow.md (137行)
- 其他任务模块：debug, review, deploy 各约40-45行
- Rust语言知识：rust.md (729行，最大文件)

### 性能瓶颈
1. **初始加载过重** - SKILL.md 包含所有核心原则
2. **按需加载未实现** - 渐进式披露原则未落地
3. **大文件阻塞** - rust.md 729行影响上下文
4. **模块边界不清** - 子文件互相引用，耦合高

---

## 优化方案

### Phase 1: 精简入口文件 (SKILL.md)

**目标**: 入口文件 < 100行，只包含高频使用内容

**操作**:
- [x] 1.1 提取 `workflow` 快捷命令列表到 SKILL.md 开头
- [x] 1.2 将完整规则移至 `core/rules_summary.md`
- [x] 1.3 创建 `core/quick_reference.md` 包含常用命令
- [x] 1.4 SKILL.md 只保留：项目概述 + 核心原则 + 常用任务表

**优化后 SKILL.md 结构** (目标 < 80行):
```markdown
---
name: workflow
description: 高效工作流 - 基于渐进式披露
---

# 快速参考

| 任务 | 命令 |
|------|------|
| 写功能 | /plan |
| 调试 | /debug |
| 代码审查 | /review |

## 核心原则
> 技能优先 + 子代理驱动 + 验证完成

## 详细文档
- 完整规则: `core/rules.md`
- TDD流程: `tasks/tdd/workflow.md`
- Rust开发: `languages/rust.md`
```

---

### Phase 2: 实现渐进式加载

**目标**: 按需加载，避免一次性读取所有文件

**操作**:
- [x] 2.1 创建 `core/on_demand/` 目录
- [x] 2.2 拆分 rules.md 为独立模块:
  - `core/on_demand/planning.md` - 规划规则
  - `core/on_demand/subagents.md` - 子代理使用
  - `core/on_demand/verification.md` - 验证规则
  - `core/on_demand/tdd_rules.md` - TDD铁律
- [x] 2.3 SKILL.md 使用条件引用

**示例 - SKILL.md 中的条件引用**:
```markdown
## 规划任务 (需要时阅读 core/on_demand/planning.md)
使用 /plan 进入规划模式...

## 子代理使用 (需要时阅读 core/on_demand/subagents.md)
常用子代理: Explore, code-reviewer, tdd-guide...
```

---

### Phase 3: 优化大文件

**目标**: 将大文件拆分为可按需加载的小模块

**操作**:
- [x] 3.1 精简 rust.md (从729行减少到约50行)
- [x] 3.2 创建 `languages/SUMMARY.md` 索引文件 (已在 rust.md 中实现)
- [x] 3.3 更新 SKILL.md 引用

---

### Phase 4: 智能缓存策略

**目标**: 根据使用频率动态调整加载策略

**操作**:
- [x] 4.1 创建 `core/usage_tracking.md` 记录访问频率
- [x] 4.2 实现分层加载策略:
  - **Layer 1** (常驻): SKILL.md (~40行)
  - **Layer 2** (常用): quick_reference.md, rules.md (~110行)
  - **Layer 3** (按需): on_demand/*.md, 语言文件 (~200行)
- [x] 4.3 语言文件 SUMMARY 索引

---

## 执行顺序

1. **Week 1**: Phase 1 - 精简 SKILL.md
2. **Week 2**: Phase 2 - 实现渐进式加载结构
3. **Week 3**: Phase 3 - 拆分大文件
4. **Week 4**: Phase 4 - 完善缓存和索引

---

## 验收标准

- [x] SKILL.md 初始加载 < 80行 (实际: 38行)
- [x] 完整规则按需加载，总上下文减少 50%+
- [x] 语言文件可独立加载
- [x] 文档可读性保持或提升

## 最终结果

| 指标 | 优化前 | 优化后 | 改进 |
|------|--------|--------|------|
| SKILL.md | 70行 | 38行 | **-46%** |
| rust.md | 729行 | 48行 | **-93%** |
| 总初始加载 | ~170行 | ~86行 | **-50%** |
| 新增模块 | - | 7个按需加载文件 | +渐进式披露 |

---

## 相关文件

- `SKILL.md` - 入口文件（优化目标）
- `core/rules.md` - 核心规则（拆分目标）
- `core/quick_reference.md` - 快速参考（新建）
- `core/on_demand/` - 按需加载模块目录（新建）
- `languages/rust.md` - Rust知识（拆分目标）
- `languages/SUMMARY.md` - 语言文件索引（新建）
