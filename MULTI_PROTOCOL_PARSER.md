# 多协议自动检测与解析系统

## 概述

本项目实现了一个统一的多协议自动检测和解析系统，支持以下电力通信协议：

1. **DL/T 645-2007** - 多功能电能表通信协议
2. **Q/CSG1209022-2019** - 计量自动化终端上行通信规约（南网）
3. **Q/CSG1209021-2019** - 计量自动化终端本地通信模块接口协议（南网）

## 项目结构

```
protocol-parser/
├── common/              # 共享工具库（BCD、校验和、字段值类型）
├── dlt645-2007/         # DLT645-2007 协议解析器
├── csg1209022/          # CSG1209022 上行通信规约解析器
├── csg-local-comm/      # CSG1209021 本地通信协议解析器
└── parser/              # 统一的多协议自动检测解析器 ⭐
```

## 核心特性

### 1. 自动协议检测

每个协议都提供了独立的检测函数：

```rust
// CSG 本地通信检测
pub fn is_csg_local_comm_frame(buf: &[u8]) -> bool

// DLT645 检测
pub fn is_dlt645_frame(buf: &[u8]) -> bool

// CSG1209022 检测
pub fn is_csg1209022_frame(buf: &[u8]) -> bool
```

**检测优先级**：
1. **CSG 本地通信** - 最特殊（检查版本位 + 保留位）
2. **DLT645** - 有明显的双 68H 特征
3. **CSG1209022** - 标准 FT1.2 帧格式

### 2. 统一解析接口

使用 `protocol-parser` crate 提供的统一接口：

```rust
use protocol_parser::{detect_protocol, auto_parse};

// 检测协议类型
match detect_protocol(&frame_bytes) {
    Some(protocol_type) => println!("识别为: {}", protocol_type.name()),
    None => println!("未知协议"),
}

// 自动解析
match auto_parse(&frame_bytes, None) {
    Ok((parsed_msg, consumed)) => {
        println!("协议: {}", parsed_msg.protocol_name());
        println!("消耗字节数: {}", consumed);
    }
    Err(e) => eprintln!("解析失败: {}", e),
}
```

### 3. 统一的结果类型

```rust
pub enum ParsedMessage {
    Dlt645(dlt645_2007::Message),
    Csg1209022(csg1209022::Message),
    CsgLocalComm(csg_local_comm::Message),
}
```

## 协议特征对比

| 特性          | CSG 本地通信       | DLT645-2007        | CSG1209022         |
|---------------|-------------------|--------------------|--------------------|
| **起始符**    | 68H               | 68H (出现2次)      | 68H                |
| **地址域**    | 可选              | 必须 (6B BCD)      | 必须 (6B BCD)      |
| **版本位**    | 必须为 00         | 无                 | 无                 |
| **保留位**    | 必须为 000        | 无                 | 无                 |
| **数据标识**  | DI (4B 小端序)    | DI (4B 小端序)     | DA + DI            |
| **数据偏移**  | 无                | +33H               | 无                 |
| **结束符**    | 16H               | 16H                | 16H                |

## 检测规则详解

### CSG 本地通信 (Q/CSG1209021-2019)

```rust
// 检测条件（优先级最高）：
1. 起始符 = 68H
2. 长度域合理
3. 结束符 = 16H
4. 控制字节版本位 (D4-D3) = 00  ⭐
5. 控制字节保留位 (D2-D0) = 000 ⭐
```

**为什么优先检测**：版本位和保留位的强约束使其特征最明显

### DLT645-2007

```rust
// 检测条件（优先级次高）：
1. 起始符 = 68H (可能有前导 FE)
2. 地址域 6 字节后
3. 第二个起始符 = 68H  ⭐⭐
4. 数据长度合理
5. 结束符 = 16H
```

**为什么次优先**：双 68H 是明显特征，不易与其他协议混淆

### CSG1209022

```rust
// 检测条件（优先级最低）：
1. 起始符 = 68H
2. 长度域合理
3. 结束符 = 16H
4. 能成功解析为 FT1.2 帧
```

**为什么最后检测**：特征最少，容易与其他协议混淆

## 使用示例

### 示例 1：基本解析

```rust
use protocol_parser::auto_parse;

let frame = vec![0x68, 0x49, 0x00, 0x40, 0x04, 0x11, ...];
match auto_parse(&frame, None) {
    Ok((msg, consumed)) => {
        println!("✓ 成功解析 {}", msg.protocol_name());
    }
    Err(e) => eprintln!("✗ 解析失败: {}", e),
}
```

### 示例 2：批量处理

```rust
use protocol_parser::{detect_protocol, auto_parse};

fn process_buffer(buffer: &[u8]) {
    let mut offset = 0;
    
    while offset < buffer.len() {
        let remaining = &buffer[offset..];
        
        // 先检测
        if detect_protocol(remaining).is_none() {
            offset += 1;
            continue;
        }
        
        // 再解析
        match auto_parse(remaining, None) {
            Ok((msg, consumed)) => {
                println!("解析到 {} 帧", msg.protocol_name());
                offset += consumed;
            }
            Err(_) => offset += 1,
        }
    }
}
```

### 示例 3：运行演示

```bash
# 运行自动解析演示
cargo run -p protocol-parser --example auto_parse_demo

# 输出示例：
# 【示例 1】CSG 本地通信帧
# 协议检测: ✓ 识别为 Q/CSG1209021-2019
# 解析结果: ✓ 成功
#   - 消耗字节数: 12
#   - AFN: 00H - 确认/否认
#   - DI: E8010001H
```

## 测试覆盖

### 全部测试

```bash
cargo test --workspace --lib
```

**测试统计**：
- ✅ csg-local-comm: 35 个测试
- ✅ csg1209022: 43 个测试
- ✅ dlt645-2007: 37 个测试
- ✅ proto-common: 12 个测试
- ✅ protocol-parser: 4 个测试
- **总计**: 131 个测试全部通过

### 协议检测测试

```bash
# 测试 CSG 本地通信检测
cargo test -p csg-local-comm is_csg_local_comm_frame

# 测试 DLT645 检测
cargo test -p dlt645-2007 is_dlt645_frame

# 测试 CSG1209022 检测
cargo test -p csg1209022 is_csg1209022_frame

# 测试统一解析器
cargo test -p protocol-parser --lib
```

## 性能特点

### 1. 零拷贝检测

协议检测不进行完整解析，只检查关键字节：

```rust
// CSG 本地通信：只检查 5 个关键位置
if buf[0] != 0x68 { return false; }  // 起始符
// ... 长度、结束符
let control = buf[3];
let version = (control >> 3) & 0x03;  // 版本位
let reserved = control & 0x07;        // 保留位
```

### 2. 早期失败

按特征明显程度排序，快速排除不匹配：

```
检测流程：
┌─────────────────┐
│ CSG 本地通信?   │ → 版本位不对 → 继续
├─────────────────┤
│ DLT645?         │ → 没有双68H → 继续
├─────────────────┤
│ CSG1209022?     │ → FT1.2解析失败 → 失败
└─────────────────┘
```

### 3. 增量解析

返回消耗的字节数，支持流式处理：

```rust
let mut offset = 0;
while offset < buffer.len() {
    match auto_parse(&buffer[offset..], None) {
        Ok((msg, consumed)) => {
            process(msg);
            offset += consumed;  // 增量推进
        }
        Err(_) => offset += 1,
    }
}
```

## 架构设计原则

### 1. 三层架构

每个协议都遵循统一的三层架构：

```
┌─────────────────────────┐
│   Engine (engine.rs)    │ ← 顶层：完整消息编解码
├─────────────────────────┤
│  Application (app/)     │ ← 应用层：AFN、DI、数据域
├─────────────────────────┤
│    Link (link/)         │ ← 链路层：帧结构、地址、控制
└─────────────────────────┘
```

### 2. 关注点分离

- **检测函数** (`is_xxx_frame`): 只检查格式，不解析内容
- **解析函数** (`decode_message`): 完整解析，返回结构化数据
- **统一接口** (`auto_parse`): 自动调度，统一结果

### 3. 错误处理

每个层次都有明确的错误类型：

```rust
// 链路层错误
pub enum LinkError {
    InvalidStartChar,
    ChecksumMismatch,
    InvalidLength,
}

// 应用层错误
pub enum AppError {
    InvalidAfn,
    InvalidDi,
    InsufficientData,
}

// 统一错误
pub enum ParseError {
    UnknownProtocol,
    Dlt645Error(String),
    Csg1209022Error(String),
    CsgLocalCommError(String),
}
```

## 扩展新协议

要添加新协议，需要：

1. **创建协议 crate**（参考现有结构）
2. **实现检测函数**：
   ```rust
   pub fn is_new_protocol_frame(buf: &[u8]) -> bool {
       // 检查协议特征
   }
   ```

3. **在 `parser/src/lib.rs` 中注册**：
   ```rust
   // 添加到 ProtocolType 枚举
   pub enum ProtocolType {
       // ... 现有协议
       NewProtocol,
   }
   
   // 添加到 detect_protocol 函数
   pub fn detect_protocol(buf: &[u8]) -> Option<ProtocolType> {
       // 按特征明显程度排序
       if new_protocol::is_new_protocol_frame(buf) {
           return Some(ProtocolType::NewProtocol);
       }
       // ... 其他协议
   }
   
   // 添加到 auto_parse 函数
   match protocol_type {
       ProtocolType::NewProtocol => {
           let (msg, consumed) = new_protocol::decode_message(buf, ...)?;
           Ok((ParsedMessage::NewProtocol(msg), consumed))
       }
       // ... 其他协议
   }
   ```

4. **添加测试**：
   ```rust
   #[test]
   fn test_detect_new_protocol() {
       let frame = vec![...];
       assert_eq!(detect_protocol(&frame), Some(ProtocolType::NewProtocol));
   }
   ```

## 实际应用场景

### 场景 1：网关设备

```rust
// 网关接收到混合协议数据流
fn gateway_handler(stream: &[u8]) {
    for (msg, consumed) in parse_stream(stream) {
        match msg {
            ParsedMessage::Dlt645(m) => route_to_meter_system(m),
            ParsedMessage::Csg1209022(m) => route_to_master_station(m),
            ParsedMessage::CsgLocalComm(m) => route_to_local_module(m),
        }
    }
}
```

### 场景 2：协议分析工具

```rust
// 协议分析器自动识别报文类型
fn analyzer(file_path: &str) {
    let data = fs::read(file_path)?;
    
    match detect_protocol(&data) {
        Some(pt) => {
            println!("检测到 {} 协议", pt.name());
            let (msg, _) = auto_parse(&data, None)?;
            display_tree(msg.to_value_tree()?);
        }
        None => println!("未知协议"),
    }
}
```

### 场景 3：测试工具

```rust
// 自动测试多协议兼容性
fn test_protocol_compatibility() {
    let test_frames = load_test_vectors();
    
    for (name, frame) in test_frames {
        match auto_parse(&frame, None) {
            Ok((msg, _)) => {
                println!("✓ {} - {}", name, msg.protocol_name());
            }
            Err(e) => {
                println!("✗ {} - {}", name, e);
            }
        }
    }
}
```

## 依赖关系

```
parser (统一解析器)
  ├── csg-local-comm
  │     ├── proto-common
  │     └── spec-engine
  ├── csg1209022
  │     ├── proto-common
  │     └── spec-engine
  └── dlt645-2007
        ├── proto-common
        └── spec-engine
```

## 文档

- [protocol-parser/README.md](parser/README.md) - 统一解析器使用指南
- [csg-local-comm/README.md](csg-local-comm/README.md) - CSG 本地通信协议详解
- [dlt645-2007/README.md](dlt645-2007/README.md) - DLT645 协议详解
- [csg1209022/README.md](csg1209022/README.md) - CSG1209022 协议详解

## 许可证

MIT License

## 总结

本项目实现了一个完整的多协议自动检测和解析系统，具有以下优势：

✅ **自动识别** - 无需手动指定协议类型  
✅ **统一接口** - 一个函数处理所有协议  
✅ **高性能** - 零拷贝检测，早期失败  
✅ **易扩展** - 清晰的架构便于添加新协议  
✅ **测试完整** - 131 个单元测试全覆盖  
✅ **文档详细** - 每个协议都有完整说明  

适用于网关设备、协议分析工具、测试系统等多种场景。
