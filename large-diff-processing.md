# 大型Diff处理方案技术文档

## 1. 问题定义

**核心问题**: 当Git diff输出过长时，AI模型会因输入超过token限制而报错，导致`ai commit`功能失效。

**典型场景**:
- 重构大型代码库
- 批量文件格式化
- 依赖更新导致的lock文件变更
- 自动生成代码的提交

**现状限制**:
- GPT-3.5/4: ~4k-8k tokens输入限制
- Claude: ~8k tokens输入限制  
- Ollama本地模型: 通常2k-4k tokens

## 2. 解决方案设计

### 2.1 整体架构

```
原始大型diff
    ↓
[长度检查] → 短diff → [原有流程]
    ↓
[按文件分段]
    ↓
[并行AI总结] → [文件级摘要]
    ↓
[整体统计] + [文件摘要] → [最终commit message]
```

### 2.2 分层处理策略

**Level 1**: 长度检查
- 阈值: 8000字符 (约2000 tokens)
- 小于阈值: 直接使用原有逻辑
- 大于阈值: 启动分段处理流程

**Level 2**: 智能分段
- 按`diff --git`边界分割
- 确保单个文件diff完整性
- 将多个文件打包成段，每段<阈值

**Level 3**: 并行处理
- 最大并发数: 3 (符合大部分API限制)
- 超时控制: 30秒/请求
- 失败策略: 快速失败，取消所有请求

## 3. 技术实现细节

### 3.1 配置扩展

```toml
[git]
# 现有配置...
max_diff_length = 8000        # diff长度阈值
max_concurrency = 3           # 最大并发数
segment_timeout_seconds = 30  # 单次请求超时
```

### 3.2 数据结构设计

```rust
// git_ops.rs
pub struct DiffSegment {
    files: Vec<String>,    // 包含的文件列表
    content: String,       // 完整的diff内容
    char_count: usize,     // 字符数统计
}

pub struct FileSummary {
    filename: String,      // 文件路径
    summary: String,       // AI生成的变更摘要
}

pub struct DiffStats {
    files_changed: usize,
    lines_added: usize,
    lines_deleted: usize,
    file_types: Vec<String>, // 主要文件类型
}
```

### 3.3 核心算法流程

**分段算法**:
```rust
fn segment_diff_by_files(diff: &str, max_length: usize) -> Vec<DiffSegment> {
    1. 按"diff --git"分割识别文件边界
    2. 逐个累加文件到当前段
    3. 当累加长度即将超过max_length时，开始新段
    4. 确保单个文件不被拆分
}
```

**并行处理**:
```rust
async fn summarize_diff_segments(segments: Vec<DiffSegment>) -> Result<Vec<FileSummary>> {
    1. 创建Semaphore(max_concurrency)限制并发
    2. 为每个segment启动async任务
    3. 使用timeout包装每个请求
    4. 任意失败时通过CancellationToken取消所有请求
    5. 收集并合并所有FileSummary结果
}
```

### 3.4 提示工程设计

**文件级总结提示**:
```
请简洁总结以下每个文件的变更(每个文件一行)：

{segment_diff}

输出格式：
filename: 变更描述 (10字以内)

示例：
src/main.rs: 添加错误处理逻辑
config.toml: 更新依赖版本
```

**最终合并提示**:
```
基于以下信息生成commit message：

统计摘要：
{diff_stats}

文件变更详情：
{file_summaries}

生成符合conventional commits格式的一行commit message。
```

### 3.5 错误处理策略

**请求失败处理**:
- 网络超时 → 显示"网络请求超时，请检查网络连接"
- API限制 → 显示"API调用受限，请稍后重试"  
- 任意segment失败 → 取消所有请求，整体失败
- 不实现重试机制，保持简单可控

**用户体验**:
- 显示处理进度: "分析大型变更 (2/5)"
- 失败时给出建设性建议: "考虑将变更拆分为多个小的commit"
- 估算处理时间: "预计需要15-30秒"

### 3.6 性能优化考量

**请求优化**:
- 合理的segment大小平衡(不能太小导致请求过多)
- 并发控制防止rate limiting
- 超时设置防止长时间等待

**成本控制**:  
- 用户可配置max_concurrency降低并发
- 只在必要时启用分段处理
- 清晰显示API调用次数让用户知情

## 4. 实现路径

1. **配置基础设施** (config.rs, 数据结构)
2. **Diff分段逻辑** (git_ops.rs核心算法)  
3. **并行AI处理** (ai_client.rs异步并发控制)
4. **主流程整合** (commit.rs调用新逻辑)
5. **用户体验优化** (进度显示，错误处理)
6. **配置文档更新** (ai.conf.toml.default)

## 5. 验收标准

- [ ] 大型diff (>8k字符) 能正常生成commit message
- [ ] 并发请求受到正确限制 (<=3)
- [ ] 单次请求失败时整体快速失败
- [ ] 用户能看到处理进度和预估时间
- [ ] 错误信息清晰且提供解决建议
- [ ] 配置项可调，适应不同用户需求

## 6. 开发进度

- [ ] 创建技术文档
- [ ] 配置基础设施
- [ ] 数据结构设计
- [ ] Diff分段算法
- [ ] 并行AI处理
- [ ] 主流程整合
- [ ] 用户体验优化
- [ ] 全面测试