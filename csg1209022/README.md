# Q/CSG1209022-2019 协议解析器

中国南方电网《计量自动化终端上行通信规约》(Q/CSG1209022-2019) Rust 实现。

## 项目概述

本项目提供了一个高性能、类型安全的协议解析器，用于解析和生成符合 Q/CSG1209022-2019 标准的计量自动化终端通信报文。

### 项目结构

```
protocol-parser/
├── common/                 # 通用工具库（BCD编码、校验和等）
├── csg1209022/            # Q/CSG1209022-2019 协议实现
│   ├── src/
│   │   ├── link/          # 链路层（帧结构、地址、控制域）
│   │   ├── app/           # 应用层（AFN、SEQ、数据单元）
│   │   │   └── body/      # 应用层数据体（DA、DI、内容解析）
│   │   ├── engine.rs      # spec-engine 桥接层
│   │   ├── field_value.rs # 字段值类型转换
│   │   ├── report.rs      # 人类可读的报文渲染
│   │   └── error.rs       # 错误类型定义
│   ├── examples/          # 示例代码
│   └── tests/             # 集成测试
└── src/                   # 命令行工具入口
```

## 核心设计原则

### 1. 分层架构

项目采用严格的分层设计，职责清晰分离：

```
┌─────────────────────────────────────────┐
│   Application (CLI/API)                 │
├─────────────────────────────────────────┤
│   Report Layer (report.rs)              │  ← 人类可读输出
├─────────────────────────────────────────┤
│   Application Layer (app/)              │  ← AFN功能码、SEQ、数据组织
│   ├── AFN routing                       │
│   ├── Body parsing (body/mod.rs)        │
│   └── Data units (DA + DI + Content)    │
├─────────────────────────────────────────┤
│   Link Layer (link/)                    │  ← FT1.2帧格式、地址、控制域
├─────────────────────────────────────────┤
│   Content Engine (spec-engine)          │  ← DI内容格式解析（外部crate）
├─────────────────────────────────────────┤
│   Common Utilities (proto-common)       │  ← BCD、校验和等基础工具
└─────────────────────────────────────────┘
```

### 2. 职责分离原则

**关键设计决策：协议帧结构 vs 数据内容**

| 组件 | 职责 | 不负责 |
|------|------|--------|
| **csg1209022 crate** | • 链路层帧结构<br>• 应用层帧结构<br>• DA、DI 顺序<br>• 时间字段解析<br>• PW 检测<br>• 循环/计数逻辑 | • DI 内容格式定义<br>• 字段级别解码 |
| **spec-engine crate** | • DI 内容格式定义<br>• 字段级别解码<br>• 数据类型转换 | • 帧结构<br>• DA、DI 排列<br>• 时间字段<br>• 循环解析 |

**为什么这样设计？**

- **spec-engine** 是通用的数据字典引擎，可以被其他协议复用
- **csg1209022** 专注于 Q/CSG1209022-2019 的特定帧结构和组织方式
- 清晰的边界避免重复实现和职责混淆

### 3. 零拷贝解析

- 解析时尽可能使用字节切片引用，避免不必要的内存分配
- 原始字节保留在 `DataUnit.raw` 和 `DataUnit.content_raw` 中，便于调试和验证

### 4. 类型安全

- 使用 Rust 类型系统编码协议约束
- 枚举类型表示有限状态（如 `Afn`、`Direction`）
- 结构体封装复杂字段（如 `DataAddress`、`SeqField`）

## 应用层数据解析设计

### 核心函数分工

| 函数 | 适用场景 | 数据格式 | PW检测 |
|------|----------|----------|--------|
| `parse_data_units()` | 大部分 AFN | `[DA + DI + 内容 [+ 时间]]` 循环 | ✅ |
| `parse_task_data()` | AFN=12H | 自描述：`DA+DI+内容+时间(5字节BCD)`<br>按任务定义：特殊格式 | ✅ |
| `parse_history_data_response()` | AFN=0DH | `[DA + DI + 内容 + 时间(6字节)]` 循环 | ✅ |
| `parse_data_identifiers()` | 下行请求 | `[DA + DI]` 循环（无内容） | ❌ |

### PW (密码域) 处理

**关键事实：**
- **长度**：固定 16 字节
- **出现位置**：除 AFN=00H 外的所有报文末尾（可选）
- **无标志位**：无法通过标志位判断是否存在，需启发式检测
- **位置**：在所有数据单元之后、Tp（时间标签）之前

**检测逻辑：**

```rust
// 当剩余字节恰好 16 字节时触发检测
if remaining == 16 {
    // 1. 尝试解析 DA
    if DA解析失败 {
        return PW;
    }
    
    // 2. 尝试解析 DI
    if DI字节不足 {
        return PW;
    }
    
    // 3. 尝试解析 DI 内容
    if 内容解析失败 {
        return PW;
    }
    
    // 4. 验证数据单元完整性
    expected_bytes = DA长度 + DI长度 + 内容长度 + [时间长度];
    if expected_bytes > 16 || (expected_bytes < 16 && expected_bytes != 16) {
        return PW;
    }
    
    // 只有恰好 16 字节时才认为是有效数据单元
}
```

**为什么在这三个函数中都需要检测？**

每个函数都独立解析数据单元序列，都可能在末尾遇到 PW：
- `parse_data_units()`: 标准循环解析
- `parse_task_data()`: AFN=12H 自描述格式
- `parse_history_data_response()`: AFN=0DH 历史数据格式

### TpV 与 with_time 的区别

**重要概念区分：**

| 字段 | 含义 | 位置 | 长度 | 控制标志 |
|------|------|------|------|----------|
| **Tp (时间标签)** | 启动帧发送时标 | 整个应用层报文**末尾** | 5 字节 | `SEQ.TpV` 位 |
| **数据时间 (DataTime)** | 数据采集时间 | 每个数据单元**末尾** | 6 字节 | AFN 格式决定 |

**处理流程：**

```rust
// ApplicationLayer::decode() 中
let tp = if seq.tpv {
    let (body_part, tp_part) = rest.split_at(rest.len() - TP_LEN);
    rest = body_part;  // ← Tp 已被移除
    Some(Tp::decode(tp_part)?)
} else {
    None
};

// 之后调用 parse_data_units()，buf 中已不包含 Tp
// with_time 参数由具体 AFN 的协议格式决定
parse_data_units(rest, protocol, region, dir, with_time)?;
```

**关键点：**
- 在调用任何数据解析函数前，Tp 已经被移除
- `with_time` 参数不是由 `TpV` 决定，而是由具体 AFN 的协议格式决定
- 大部分 AFN 的 `with_time` 为 `false`，特殊 AFN 有自己的时间处理逻辑

## 特殊 AFN 处理

### AFN=12H (读任务数据)

**上行响应格式：**
```
DA(2) + DI(4任务号) + 数据结构方式(1) + 任务数据内容
```

**数据结构方式：**
- **0 (自描述)**：`n(1) + m(1) + [DA+DI+内容+时间(5字节BCD)]×(n×m)`
  - n: 信息点标识组数
  - m: 数据标识编码组数
  - 时间格式：5 字节 BCD（YYMMDDhhmm）
- **1 (按任务定义)**：需要预先知道任务参数配置（暂未完全实现）

**参考：** 表 A.10.4 (自描述任务数据格式)、表 A.10.5 (任务定义数据格式)

### AFN=0DH (读历史数据)

**上行响应格式：**
```
[DA + DI + 内容 + 数据时间(6字节)] 循环直到缓冲区结束
```

**关键特点：**
- 同一 DA+DI 可以重复多次（不同时间点）
- 按时间先后顺序组织
- 无"内容数 N"字段（早期实现错误已修复）
- 基于长度的循环解析

**参考：** 6.2.7.2.1

### AFN=13H (读告警数据)

**上行响应格式：**
```
[DA + DI + 告警内容] 循环
```

**关键特点：**
- 标准格式，使用 `parse_data_units()`
- **无"告警条数 N"字段**（早期假设错误）
- 告警时间包含在告警内容内部，不是单独字段
- 同一 DA+DI 代表同一告警数据标识

**参考：** 6.2.8.2

### AFN=00H (确认/否认)

**特殊性：**
- 这是**唯一不可能包含 PW** 的 AFN
- 简单的 DA+DI+内容格式

## 已修复的问题

### 问题 1：错误的"记录数"字段假设

**问题描述：**
早期实现假设 AFN=0DH 和 AFN=13H 的格式为：
```
DA + DI + N(记录数1字节) + [内容]×N
```

**实际格式：**
```
[DA + DI + 内容 [+ 时间]] 循环直到缓冲区结束
```

**修复方案：**
- 移除错误的记录数读取
- 改为基于长度的循环解析
- 添加 PW 检测来判断何时停止

**受影响函数：**
- `parse_history_data_response()` (AFN=0DH)
- `parse_data_units()` 用于 AFN=13H

### 问题 2：缺少 PW 检测

**问题描述：**
原实现在循环解析时会尝试将 PW（16字节密码域）当作数据单元解析，导致：
- 解析错误
- 数据单元数量不正确
- 可能的安全问题

**修复方案：**
在三个核心解析函数中添加完整的 PW 启发式检测：
1. `parse_data_units()`
2. `parse_task_data()`
3. `parse_history_data_response()`

**检测策略：**
当剩余字节恰好 16 字节时：
- 尝试完整解析一个数据单元
- 验证所需字节数
- 如果不匹配则认为是 PW

### 问题 3：AFN=12H 未实现自描述格式

**问题描述：**
原实现对 AFN=12H 使用通用的 `parse_data_units()`，未处理：
- 数据结构方式字节
- n×m 计数
- 5 字节 BCD 时间格式

**修复方案：**
- 创建专用的 `parse_task_data()` 函数
- 实现自描述格式解析（data_structure=0）
- 处理 5 字节 BCD 时间（YYMMDDhhmm）

### 问题 4：容错处理不足

**问题描述：**
解析失败时立即返回错误，导致部分已解析数据丢失。

**修复方案：**
- 解析失败时优雅停止，返回已成功解析的数据单元
- 只有在完全无法解析时才返回错误
- 保留 `offset` 信息供调用者判断

## 开发规范

### 1. 添加新 AFN 支持

**步骤：**

1. **确定 AFN 的数据格式**
   - 查阅协议文档（skill 中的 references/）
   - 确认是否有特殊的帧结构
   - 确认是否需要数据时间字段

2. **选择合适的解析函数**
   - 标准 DA+DI+内容 → 使用 `parse_data_units()`
   - 特殊格式 → 创建专用解析函数

3. **在 `ApplicationLayer::decode()` 中添加路由**
   ```rust
   (Afn::NewFunction, Direction::Up) => {
       let (units, _) = body::parse_xxx(rest, protocol, region, dir)?;
       ApplicationBody::NewFunctionResponse(units)
   }
   ```

4. **考虑 PW 检测**
   - 如果使用循环解析，必须考虑 PW 检测
   - 除 AFN=00H 外都可能有 PW

5. **添加测试**
   - 在 `tests/` 中添加真实报文测试
   - 验证 PW 检测逻辑

### 2. 修改解析逻辑的注意事项

**禁止事项：**

❌ **在 spec-engine 中处理帧结构**
```rust
// 错误：spec-engine 不应该知道 DA、DI、时间的排列
spec_engine::parse_full_data_unit(...); // 不要这样做
```

❌ **在 csg1209022 中重复实现字段解码**
```rust
// 错误：字段解码应该委托给 spec-engine
fn parse_di_content_manually(di: u32, buf: &[u8]) -> Value {
    match di {
        0x00010001 => { /* 手动解码... */ } // 不要这样做
    }
}
```

❌ **假设存在协议中未定义的字段**
```rust
// 错误：协议中没有记录数字段，不要假设
let count = buf[0]; // 除非协议明确定义
for _ in 0..count { ... }
```

**推荐做法：**

✅ **基于长度的循环解析**
```rust
while offset < buf.len() {
    // 尝试解析，失败则停止
    if let Ok(unit) = parse_one_unit(&buf[offset..]) {
        units.push(unit);
        offset += consumed;
    } else {
        break;
    }
}
```

✅ **委托给 spec-engine**
```rust
let (raw_value, content_consumed) = 
    spec_engine::parse_di(protocol, di, region, dir, &buf[offset..])?;
let value = field_value::from_spec_engine(&raw_value);
```

✅ **检查协议文档**
```rust
// 添加注释引用协议章节
// 参考协议 6.2.7.2.1：按时间先后顺序组织
```

### 3. PW 检测实现模板

当实现新的循环解析函数时，使用以下模板：

```rust
pub fn parse_xxx_response(
    buf: &[u8],
    protocol: &str,
    region: &str,
    dir: Option<&str>,
) -> Result<(Vec<DataUnit>, usize)> {
    let mut offset = 0;
    let mut units = Vec::new();
    const PW_LEN: usize = 16;

    while offset < buf.len() {
        let remaining = buf.len() - offset;
        
        // PW 检测
        if remaining == PW_LEN {
            let mut is_pw = false;
            
            if let Ok((_da, da_consumed)) = DataAddress::decode(&buf[offset..]) {
                let test_offset = offset + da_consumed;
                
                if remaining >= da_consumed + DI_LEN {
                    let di = u32::from_le_bytes([
                        buf[test_offset],
                        buf[test_offset + 1],
                        buf[test_offset + 2],
                        buf[test_offset + 3],
                    ]);
                    let content_offset = test_offset + DI_LEN;
                    
                    if let Ok((_, content_consumed)) = spec_engine::parse_di(
                        protocol, di, region, dir, &buf[content_offset..]
                    ) {
                        // 根据具体格式计算 expected_consumed
                        let expected_consumed = da_consumed + DI_LEN + content_consumed + TIME_LEN;
                        
                        if expected_consumed > PW_LEN || 
                           (expected_consumed < PW_LEN && /* 其他条件 */) {
                            is_pw = true;
                        }
                    } else {
                        is_pw = true;
                    }
                } else {
                    is_pw = true;
                }
            } else {
                is_pw = true;
            }
            
            if is_pw {
                break;
            }
        }
        
        // 正常解析逻辑
        // ...
    }

    Ok((units, offset))
}
```

### 4. 错误处理原则

**渐进式错误处理：**

```rust
// 1. 尝试解析
match parse_something(&buf[offset..]) {
    Ok((value, consumed)) => {
        // 成功，继续
        offset += consumed;
    }
    Err(e) => {
        // 2. 如果已经有部分成功结果，返回部分结果
        if !units.is_empty() {
            return Ok((units, offset));
        }
        // 3. 否则返回错误
        return Err(e);
    }
}
```

**缓冲区检查：**

```rust
// 在解析前检查缓冲区大小
if buf.len() < offset + REQUIRED_LEN {
    // 不足以构成完整单元，优雅停止
    break;
}
```

### 5. 测试要求

**每个 AFN 至少需要：**

1. **正常报文测试**
   ```rust
   #[test]
   fn test_afn_xx_normal() {
       let bytes = hex!("68 ...");
       let frame = Frame::decode(&bytes).unwrap();
       assert_eq!(frame.app.afn, Afn::ReadXxx);
       assert_eq!(units.len(), expected);
   }
   ```

2. **带 PW 的报文测试**
   ```rust
   #[test]
   fn test_afn_xx_with_pw() {
       let bytes = hex!("68 ... [16 bytes PW] 16");
       let frame = Frame::decode(&bytes).unwrap();
       // 验证 PW 没有被当作数据单元
   }
   ```

3. **边界条件测试**
   ```rust
   #[test]
   fn test_afn_xx_incomplete() {
       let bytes = hex!("68 ... [incomplete]");
       // 应该返回已解析的部分，不崩溃
   }
   ```

## 常见问题排查

### 问题：解析的数据单元数量不对

**可能原因：**
1. PW 被误当作数据单元
2. 循环终止条件不正确
3. 协议格式理解错误（如假设有记录数字段）

**排查步骤：**
1. 检查剩余字节是否恰好 16 字节
2. 启用详细日志查看解析过程
3. 对照协议文档确认格式

### 问题：PW 检测误判

**可能原因：**
1. 真实数据单元恰好 16 字节
2. expected_consumed 计算错误

**排查步骤：**
1. 手动计算数据单元字节数
2. 验证 DI 内容长度
3. 检查时间字段长度

### 问题：spec-engine 解析失败

**可能原因：**
1. DI 在字典中不存在
2. 数据格式不匹配
3. region 或 protocol 参数错误

**排查步骤：**
1. 确认 DI 是否在协议附录中定义
2. 检查 protocol="csg13", region="南网"
3. 查看 spec-engine 日志

## 参考文档

- **协议规范**：`.kiro/skills/csg1209022-metering-protocol/references/`
  - `02-应用层通用格式.md`: 应用层基础结构
  - `03a-AFN功能码-00H至0EH.md`: AFN 功能码定义
  - `04-附录A-终端参数格式.md`: 数据标识格式

- **AFN 实现总结**：`AFN_IMPLEMENTATION_SUMMARY.md`

- **示例代码**：`csg1209022/examples/`
  - `parse_user_frame.rs`: 解析自定义报文
  - `parse_afn12_self_desc.rs`: AFN=12H 自描述格式示例

## 许可证

本项目采用 MIT 许可证。详见 LICENSE 文件。
