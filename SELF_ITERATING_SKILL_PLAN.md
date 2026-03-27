# 自我迭代 Skill 系统设计

## 目标

将 workflow skill 打造成一个能够**自我进化**的系统：
- 自动追踪使用情况
- 分析效果并优化
- 提取学习新模式
- 持续改进自身内容

---

## 核心架构

```
workflow/
├── SKILL.md              # 入口 (不变)
├── core/
│   ├── self_iteration/   # 自我迭代核心
│   │   ├── tracker.rs    # 使用追踪
│   │   ├── analyzer.rs   # 效果分析
│   │   ├── optimizer.rs  # 内容优化
│   │   └── learner.rs    # 模式学习
│   ├── memory/           # 记忆存储
│   │   ├── usage.json    # 使用记录
│   │   ├── patterns.md   # 提取的模式
│   │   └── insights.md   # 洞察总结
│   └── evolution/         # 进化引擎
│       ├── evaluator.rs  # 效果评估
│       └── mutator.rs    # 变更生成
```

---

## Phase 1: 使用追踪系统

### 1.1 追踪哪些数据

- 调用频率：哪些模块被频繁引用
- 完成任务：哪些工作流成功完成
- 用户反馈：手动标记的有效/无效
- 时间统计：每个任务耗时

### 1.2 数据结构

```json
{
  "usage": {
    "tasks/tdd/workflow.md": 45,
    "core/on_demand/planning.md": 30,
    "core/on_demand/tdd_rules.md": 25
  },
  "completed_tasks": [
    {"type": "feature", "duration": "15m", "success": true}
  ],
  "feedback": []
}
```

---

## Phase 2: 效果分析引擎

### 2.1 分析维度

| 维度 | 指标 |
|------|------|
| **活跃度** | 模块被调用次数 |
| **有效性** | 完成任务成功率 |
| **相关性** | 任务类型与模块匹配度 |
| **时效性** | 内容更新频率 |

### 2.2 评估算法

```
score = (completions / calls) * relevance * recency
```

---

## Phase 3: 自动优化器

### 3.1 优化策略

| 问题 | 优化动作 |
|------|---------|
| 使用率低 | 精简或合并到其他模块 |
| 内容过时 | 标记需要更新 |
| 重复内容 | 合并到统一位置 |
| 缺失内容 | 建议添加新模块 |

### 3.2 优化规则

```
IF usage_count < threshold AND size > max_size:
    → 精简内容

IF content_age > 90 days:
    → 标记需要复习

IF duplicate_content_found:
    → 合并重复
```

---

## Phase 4: 模式学习器

### 4.1 学习来源

- Git 提交历史 → 提取有效的开发模式
- 完成的对话 → 分析成功的工作流
- 用户反馈 → 标记改进点
- 外部最佳实践 → 定期同步

### 4.2 提取流程

```
1. 收集数据源 (commits, conversations)
2. 识别重复模式
3. 生成模式文档
4. 建议集成到 skill
```

---

## Phase 5: 进化引擎

### 5.1 进化触发条件

- 手动触发: `/evolve`
- 自动触发: 每周或完成 N 个任务后
- 按需触发: 使用率明显下降时

### 5.2 进化动作

1. **小进化** (90% 场景)
   - 更新使用统计
   - 调整模块权重
   - 优化引用链接

2. **中进化** (9% 场景)
   - 合并小模块
   - 拆分过大模块
   - 更新示例代码

3. **大进化** (1% 场景)
   - 重构整体结构
   - 添加全新模块
   - 重大版本升级

---

## 核心命令

```bash
/skills list              # 列出 skills
/workflow:status          # 查看自身状态
/workflow:stats           # 查看使用统计
/workflow:analyze         # 分析效果
/workflow:evolve          # 触发进化
/workflow:learn           # 学习新模式
```

---

## 验收标准

- [ ] 能够追踪模块使用情况
- [ ] 能够分析哪些内容有效/无效
- [ ] 能够自动生成优化建议
- [ ] 能够提取并学习新模式
- [ ] 手动/自动触发进化
