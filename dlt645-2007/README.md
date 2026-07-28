# DL/T 645-2007 多功能电能表通信协议解析器

本库实现了 **DL/T 645-2007《多功能电能表通信协议》**的完整解析功能，采用与 CSG1209022 一致的三层架构设计。

## 架构设计

### 设计目标

1. **与 CSG1209022 保持架构一致**：便于调用者统一处理不同协议
2. **清晰的职责分层**：链路层、应用层、顶层引擎分工明确
3. **集成 spec-engine**：利用 spec-engine 解析 DI（数据标识）对应的具体内容
4. **类型安全**：避免直接暴露外部依赖（spec-engine）的类型到公开 API

### 三层架构

```
┌─────────────────────────────────────────────────────────────┐
│                    顶层引擎 (engine)                          │
│  - decode_message(): 完整报文解析（链路层+应用层）            │
│  - encode_message(): 完整报文编码                             │
│  - Message { frame, application }                           │
└──────────────────────┬──────────────────────────────────────┘
                       │
         ┌─────────────┴──────────────┐
         │                            │
┌────────▼─────────┐         ┌────────▼──────────┐
│  链路层 (link)    │         │  应用层 (app)      │
│  - Frame          │         │  - ApplicationLayer│
│  - Address        │         │  - ApplicationBody │
│  - ControlCode    │         │  - DataItem        │
│  - 处理+33H偏移   │         │  - 调用spec-engine │
└───────────────────┘         └────────────────────┘
```

### 模块结构

| 模块 | 职责 | 关键类型 | 文件位置 |
|-----|------|---------|---------|
| **链路层 (link)** | 帧格式、地址、控制码、校验和、数据偏移 | `Frame` | `src/link/` |
| **应用层 (app)** | 数据域解析、DI处理、spec-engine集成 | `ApplicationLayer`, `ApplicationBody` | `src/app/` |
| **顶层引擎 (engine)** | 串联链路层和应用层 | `Message`, `decode_message()` | `src/engine.rs` |
| **地址** | 6字节BCD地址编解码 | `Address` | `src/address.rs` |
| **控制码** | 方向、功能码、状态 | `ControlCode`, `FunctionCode` | `src/control.rs` |
| **数据标识** | 4字节DI编解码 | `DataIdentifier` | `src/data_identifier.rs` |
| **字段值** | spec-engine解析结果封装 | `FieldValue` | `src/field_value.rs` |
| **错误** | 错误类型定义 | `Error`, `Result` | `src/error.rs` |

## 协议概述

### 帧格式

```
[FE FE FE FE] | 68H | A0~A5 | 68H | C | L | DATA(L字节) | CS | 16H
  前导(可选)         └─地址6字节─┘         └──数据域─────┘
```

**关键特性：**
- **前导字节（可选）**：4个 `FE`，用于唤醒从站
- **地址域**：6字节BCD编码，低字节先传，表示12位十进制地址
- **控制码 (C)**：1字节，包含方向、状态、后续标志和功能码
- **数据长度 (L)**：1字节，表示数据域字节数
- **数据偏移**：DATA域所有字节**发送时+33H，接收时-33H**（链路层自动处理）
- **数据标识 (DI)**：4字节 DI3DI2DI1DI0（低字节先传）
- **校验和 (CS)**：从第一个68H到DATA末尾的累加和（mod 256）

### 控制码结构

```
D7      D6         D5            D4~D0
方向    从站状态   后续数据帧    功能码(5位)
```

- **D7=0**：主站→从站（下行/请求）
- **D7=1**：从站→主站（上行/响应）
- **D6=0**：从站正常应答
- **D6=1**：从站异常应答（错误响应）
- **D5=1**：有后续数据帧
- **D4~D0**：功能码（5位，0x00~0x1F）

### 主要功能码

| 功能码 | 说明 | 方向 | 实现状态 |
|--------|------|------|----------|
| `08H` | 广播校时 | 下行 | ✅ 已实现 |
| `11H` | 读数据 | 双向 | ✅ 已实现 |
| `12H` | 读后续数据 | 双向 | ✅ 已实现 |
| `14H` | 写数据 | 双向 | ✅ 已实现 |
| `16H` | 冻结命令 | 下行 | ✅ 已实现 |
| `17H` | 更改通信速率 | 下行 | ✅ 已实现 |
| `18H` | 修改密码 | 双向 | ✅ 已实现 |
| `19H` | 最大需量清零 | 双向 | ✅ 已实现 |
| `1AH` | 电表清零 | 双向 | ✅ 已实现 |
| `1BH` | 事件清零 | 双向 | ✅ 已实现 |

## 使用示例

### 基础示例：完整报文解析（使用 engine）

```rust
use dlt645_2007::engine::decode_message;

// 真实报文字节
let bytes = vec![
    0x68, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x68,
    0x91, 0x12, 0x32, 0x38, 0x33, 0x37, 0x33, 0x34,
    // ... 更多字节
];

// 解析完整报文（链路层 + 应用层 + spec-engine）
let (msg, consumed) = decode_message(&bytes, "dlt645-2007", "国网")?;

// 访问链路层信息
println!("地址: {}", msg.frame.address.to_decimal_string());
println!("方向: {:?}", msg.frame.control.direction);

// 访问应用层信息
println!("功能码: {:?}", msg.application.function);

// 访问解析后的数据（spec-engine）
if let ApplicationBody::ReadResponse { items } = &msg.application.body {
    for item in items {
        println!("DI: {:08X}H", item.identifier.to_u32());
        // spec-engine 解析结果
        if let Some(parsed) = &item.parsed_value {
            println!("  解析结果: {:?}", parsed);
        }
    }
}
```

### 构造并编码报文

```rust
use dlt645_2007::{
    engine::encode_message,
    link::Frame,
    Address, ApplicationBody, ApplicationLayer, ControlCode,
    DataIdentifier, FunctionCode,
};

// 构造链路层帧
let frame = Frame {
    address: Address::from_decimal_str("123456789012")?,
    control: ControlCode::master_request(FunctionCode::ReadData),
    payload: Vec::new(), // 会被应用层覆盖
};

// 构造应用层
let application = ApplicationLayer {
    function: FunctionCode::ReadData,
    body: ApplicationBody::ReadRequest {
        identifiers: vec![DataIdentifier::from_bytes([0x00, 0x00, 0x00, 0x00])],
    },
};

// 编码为完整字节序列
let bytes = encode_message(&frame, &application, false);
```

### 广播校时

```rust
use dlt645_2007::{
    engine::encode_message,
    link::Frame,
    Address, ApplicationBody, ApplicationLayer, BroadcastTimeData,
    ControlCode, FunctionCode,
};

let frame = Frame {
    address: Address::broadcast(), // 999999999999
    control: ControlCode::master_request(FunctionCode::BroadcastTime),
    payload: Vec::new(),
};

let application = ApplicationLayer {
    function: FunctionCode::BroadcastTime,
    body: ApplicationBody::BroadcastTime {
        time: BroadcastTimeData {
            second: 0x07,  // BCD: 07秒
            minute: 0x08,  // BCD: 08分
            hour: 0x14,    // BCD: 14时（20时）
            day: 0x15,     // BCD: 15日
            month: 0x03,   // BCD: 03月
            year: 0x24,    // BCD: 24年（2024）
        },
    },
};

// 编码时包含前导字节（用于唤醒从站）
let bytes = encode_message(&frame, &application, true);
```

## 设计要点与注意事项

### 1. **架构一致性要求** ⚠️ 重要

**必须与 CSG1209022 保持一致的架构**：

- ✅ 三层结构：link → app → engine
- ✅ `ApplicationLayer::decode` 签名一致
- ✅ `Direction` 参数正确传递给 spec-engine
- ✅ `FieldValue` 类型定义一致（封装 spec-engine 的 Value）

**关键差异说明**：

| 项目 | CSG1209022 | DLT645-2007 | 原因 |
|-----|-----------|------------|------|
| ApplicationLayer::decode 参数 | `(payload, direction, protocol, region)` | `(payload, function, direction, is_error, protocol, region)` | DLT645 需要功能码和错误标志来区分数据格式 |
| Direction 枚举 | `Down`/`Up` | `MasterToSlave`/`SlaveToMaster` | 协议术语不同，但语义相同 |
| spec-engine dir 参数 | `"0"`=下行, `"1"`=上行 | 同左 | spec-engine 统一约定 |

### 2. **数据偏移处理** ⚠️ 核心特性

DL/T 645 协议要求数据域所有字节传输时+33H，接收时-33H。

**处理位置**：链路层 `link/frame.rs`

```rust
// 编码时自动添加偏移
fn apply_offset(data: &[u8]) -> Vec<u8> {
    data.iter().map(|&b| b.wrapping_add(0x33)).collect()
}

// 解码时自动移除偏移
fn remove_offset(data: &[u8]) -> Vec<u8> {
    data.iter().map(|&b| b.wrapping_sub(0x33)).collect()
}
```

**注意**：
- ✅ `Frame::encode` 和 `Frame::decode` **自动处理**偏移
- ✅ `Frame.payload` 已经是**去除偏移后**的原始数据
- ❌ **应用层不需要也不应该**再次处理偏移
- ❌ **用户代码不需要**手动处理偏移

### 3. **地址字节序** ⚠️ 易错点

地址域采用**低字节先传**，但地址**数值表示高字节在前**。

**示例**：地址 `123456789012`

1. 十进制分解：`12-34-56-78-90-12`（从高位到低位）
2. BCD编码：`[0x12, 0x34, 0x56, 0x78, 0x90, 0x12]`（A5~A0）
3. 传输顺序：`[0x12, 0x90, 0x78, 0x56, 0x34, 0x12]`（A0~A5，低字节先传）

**API 使用**：
```rust
// ✅ 正确：使用十进制字符串
let addr = Address::from_decimal_str("123456789012")?;

// ✅ 正确：从字节数组解析（自动处理字节序）
let (addr, _) = Address::decode(&[0x12, 0x90, 0x78, 0x56, 0x34, 0x12])?;

// ❌ 错误：直接构造字节数组（会导致字节序错误）
// 不要这样做！
```

### 4. **spec-engine 集成** ⚠️ 关键设计

**设计原则**：
- 只在**应用层**调用 spec-engine
- **链路层**只负责提取原始 payload（已去除+33H）
- 使用 `FieldValue` 封装 spec-engine 的 `Value`，避免直接暴露外部依赖

**调用流程**：

```
decode_message()
  ↓
Frame::decode()  // 链路层：解析帧格式、去除+33H
  ↓
ApplicationLayer::decode()  // 应用层：解析数据域
  ↓
ApplicationBody::parse()  // 识别功能码，提取 DI + 数据
  ↓
parse_di_with_spec_engine()  // 调用 spec-engine 解析 DI 内容
  ↓
spec_engine::parse_di(protocol, di, region, dir, data)
  ↓
from_spec_engine()  // 转换为 FieldValue
```

**Direction 参数传递**：

```rust
// ✅ 正确：转换为 spec-engine 约定的字符串
let dir: Option<&str> = match direction {
    Direction::MasterToSlave => Some("0"),  // 下行/请求
    Direction::SlaveToMaster => Some("1"),  // 上行/响应
};

// ❌ 错误：传递 None 或其他值
let dir = None;  // spec-engine 无法区分请求/响应格式
let dir = Some("down");  // spec-engine 不识别此值
```

### 5. **FieldValue 封装** ⚠️ API 稳定性

**为什么需要 FieldValue**：

`spec-engine` 是外部依赖，其 `Value` 类型可能随时变化（添加字段、变体）。如果直接暴露 `spec_engine::Value` 到公开 API，会导致：
- spec-engine 每次更新都是破坏性变更
- 下游代码被迫跟着更新

**解决方案**：

```rust
// ✅ 正确：定义自己的 FieldValue
pub enum FieldValue {
    Int(i64),
    Float(f64),
    Str(String),
    // ...
}

// ✅ 正确：只在边界转换
pub fn from_spec_engine(value: &SpecValue) -> FieldValue {
    match value {
        SpecValue::Int(i) => FieldValue::Int(*i),
        // ... 穷尽匹配（不使用 _ => ）
    }
}
```

**注意**：
- ✅ `from_spec_engine` 必须**穷尽匹配**所有变体
- ✅ 不使用 `_ =>` 兜底分支
- ✅ spec-engine 新增变体时编译会失败，强制我们处理
- ❌ 不要在公开 API 中直接返回 `spec_engine::Value`

### 6. **校验和计算** ⚠️ 范围界定

校验和 = **从第一个68H到数据域末尾**的字节累加和（mod 256）

**包含**：
- ✅ 第一个 68H
- ✅ 地址域（6字节）
- ✅ 第二个 68H
- ✅ 控制码 C
- ✅ 数据长度 L
- ✅ 数据域（L字节，带+33H偏移）

**不包含**：
- ❌ 前导字节（FE FE FE FE）
- ❌ 校验和本身（CS）
- ❌ 结束符（16H）

```rust
// ✅ 正确的校验和计算
let cs = bytes[0..len-2].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));

// ❌ 错误：包含了前导字节
let cs = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
```

### 7. **错误处理**

**从站异常应答**：
- 控制码 D6=1
- 数据域长度=1字节
- 数据域内容=错误位（每一位代表一种错误）

```rust
// 检测错误响应
if frame.control.is_error() {
    if let ApplicationBody::Error { error_bits } = &application.body {
        if error_bits & 0b00000010 != 0 {
            println!("数据不存在");
        }
    }
}
```

**错误位定义**：
```
Bit 0: 其他错误
Bit 1: 数据不存在
Bit 2: 密码错误/未授权
Bit 3: 从站忙
Bit 4: 年时段超
Bit 5: 日时段超
Bit 6: 费率数超
Bit 7: 保留
```

### 8. **开发规范** ⚠️ 必读

**添加新功能时**：

1. ✅ **先检查 CSG1209022** 的对应实现
2. ✅ **保持架构一致**：不要偏离三层设计
3. ✅ **更新测试**：每个新功能都要有测试
4. ✅ **更新文档**：同步更新 README.md

**修改现有代码时**：

1. ✅ **运行所有测试**：`cargo test -p dlt645-2007`
2. ✅ **检查 CSG1209022**：确保两个项目保持一致
3. ✅ **验证 spec-engine 集成**：确保 direction 参数正确传递
4. ✅ **更新 README**：记录设计决策

**禁止的操作**：

- ❌ 不要在应用层再次处理+33H偏移（链路层已处理）
- ❌ 不要直接暴露 `spec_engine::Value` 到公开 API
- ❌ 不要手动构造地址字节数组（使用 `Address::from_decimal_str`）
- ❌ 不要在 `from_spec_engine` 中使用 `_ =>` 兜底分支
- ❌ 不要跳过 `direction` 参数（必须传递给 spec-engine）

## 运行示例

```bash
# 简单解析示例
cargo run --example simple_parse

# 真实报文解析（带 spec-engine）
cargo run --example parse_real_frame

# 运行所有测试
cargo test -p dlt645-2007

# 运行集成测试
cargo test -p dlt645-2007 --test integration
```

## 测试覆盖

- ✅ 链路层：地址、控制码、帧编解码、校验和
- ✅ 应用层：各类功能码、数据项解析
- ✅ 引擎层：完整报文往返测试
- ✅ spec-engine 集成：真实报文解析验证
- ✅ 错误处理：无效帧、校验和错误、长度不足
- ✅ 边界情况：广播地址、前导字节、后续帧

**测试统计**：44 个单元测试 + 11 个集成测试

## 与 CSG1209022 的关系

DL/T 645-2007 可作为 **Q/CSG1209022 AFN=10H（中继转发）**的内层协议：

```
CSG1209022 帧
└─ AFN=10H (中继转发)
   └─ 中继报文内容 = DL/T 645 完整帧
```

两个协议采用相同的架构设计，可以：
- 统一的 `Message { frame, application }` 结构
- 统一的 `FieldValue` 类型
- 统一的 spec-engine 集成方式

## 许可证

MIT License

## 参考资料

- DL/T 645-2007《多功能电能表通信协议》
- Q/CSG1209022-2019《计量自动化终端上行通信规约》
- spec-engine 文档
- 本项目 CSG1209022 实现（架构参考）
