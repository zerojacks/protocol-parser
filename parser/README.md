# Protocol Parser - 统一多协议解析器

自动检测并解析多种电力通信协议的统一接口。

## 支持的协议

- **DL/T 645-2007**: 多功能电能表通信协议
- **Q/CSG1209022-2019**: 计量自动化终端上行通信规约（南网）
- **Q/CSG1209021-2019**: 计量自动化终端本地通信模块接口协议（南网）

## 核心功能

### 1. 自动协议检测

根据帧格式特征自动识别协议类型：

```rust
use protocol_parser::detect_protocol;

let frame_bytes = vec![0x68, 0x49, 0x00, 0x40, ...];
match detect_protocol(&frame_bytes) {
    Some(protocol_type) => println!("识别为: {}", protocol_type.name()),
    None => println!("未知协议"),
}
```

**检测优先级**：
1. Q/CSG1209021 (本地通信) - 最特殊，检查版本位和保留位
2. DL/T 645-2007 - 有明显的双 68H 特征
3. Q/CSG1209022 - 标准 FT1.2 帧格式

### 2. 自动解析

自动检测协议并调用对应的解析器：

```rust
use protocol_parser::auto_parse;

let frame_bytes = vec![0x68, 0x49, 0x00, 0x40, ...];
match auto_parse(&frame_bytes, None) {
    Ok((parsed_msg, consumed)) => {
        println!("协议: {}", parsed_msg.protocol_name());
        println!("消耗字节数: {}", consumed);
        
        // 根据协议类型处理
        match parsed_msg {
            ParsedMessage::CsgLocalComm(msg) => { /* ... */ },
            ParsedMessage::Dlt645(msg) => { /* ... */ },
            ParsedMessage::Csg1209022(msg) => { /* ... */ },
        }
    }
    Err(e) => eprintln!("解析失败: {}", e),
}
```

### 3. 统一结果类型

所有协议的解析结果都包装在 `ParsedMessage` 枚举中：

```rust
pub enum ParsedMessage {
    Dlt645(dlt645_2007::Message),
    Csg1209022(csg1209022::Message),
    CsgLocalComm(csg_local_comm::Message),
}
```

提供统一的接口：
- `protocol_type()` - 获取协议类型枚举
- `protocol_name()` - 获取协议名称字符串
- `to_value_tree()` - 转换为树形结构（用于前端展示）

## 使用示例

### 基本用法

```rust
use protocol_parser::auto_parse;

fn parse_frame(bytes: &[u8]) {
    match auto_parse(bytes, None) {
        Ok((msg, consumed)) => {
            println!("✓ 成功解析 {} 协议", msg.protocol_name());
            println!("  消耗字节数: {}", consumed);
        }
        Err(e) => {
            eprintln!("✗ 解析失败: {}", e);
        }
    }
}
```

### 批量解析

```rust
use protocol_parser::{detect_protocol, auto_parse};

fn parse_multiple_frames(buffer: &[u8]) -> Vec<ParsedMessage> {
    let mut results = Vec::new();
    let mut offset = 0;

    while offset < buffer.len() {
        let remaining = &buffer[offset..];
        
        // 先检测协议类型
        if detect_protocol(remaining).is_none() {
            offset += 1; // 跳过无效字节
            continue;
        }

        // 解析帧
        match auto_parse(remaining, None) {
            Ok((msg, consumed)) => {
                results.push(msg);
                offset += consumed;
            }
            Err(_) => {
                offset += 1;
            }
        }
    }

    results
}
```

## 协议检测规则

### Q/CSG1209021 本地通信

- 起始符：`68H`
- 控制字节版本位（D4-D3）必须为 `00`
- 保留位（D2-D0）必须为 `000`
- 结束符：`16H`

### DL/T 645-2007

- 第一个起始符：`68H`（可能有前导 `FE`）
- 地址域：6字节
- 第二个起始符：`68H`
- 结束符：`16H`

### Q/CSG1209022

- 起始符：`68H`
- 符合 FT1.2 帧格式
- 结束符：`16H`

## 运行示例

项目包含完整的演示示例：

```bash
cargo run -p protocol-parser --example auto_parse_demo
```

输出示例：

```
=== 多协议自动解析示例 ===

【示例 1】CSG 本地通信帧
原始字节 (12 bytes): [68, 0C, 00, 80, 00, 01, 01, 00, 01, E8, 6B, 16]
协议检测: ✓ 识别为 Q/CSG1209021-2019
解析结果: ✓ 成功
  - 消耗字节数: 12
  - 协议名称: Q/CSG1209021-2019
  - AFN: 00H - 确认/否认
  - SEQ: 1
  - DI: E8010001H
```

## 测试

运行所有测试：

```bash
# 测试统一解析器
cargo test -p protocol-parser --lib

# 测试各协议的检测函数
cargo test -p csg-local-comm is_csg_local_comm_frame
cargo test -p dlt645-2007 is_dlt645_frame
cargo test -p csg1209022 is_csg1209022_frame
```

## 错误处理

解析可能失败的情况：

```rust
pub enum ParseError {
    /// 无法识别协议类型
    UnknownProtocol,
    
    /// 特定协议解析错误
    Dlt645Error(String),
    Csg1209022Error(String),
    CsgLocalCommError(String),
    
    /// 渲染错误
    RenderError(String),
    
    /// 功能未实现
    NotImplemented(String),
}
```

## 架构设计

```
protocol-parser/
├── src/
│   └── lib.rs              # 统一接口
├── examples/
│   └── auto_parse_demo.rs  # 完整示例
└── Cargo.toml

依赖的协议实现：
├── dlt645-2007/            # DLT645 协议
├── csg1209022/             # 南网上行通信规约
└── csg-local-comm/         # 南网本地通信协议
```

每个协议 crate 都导出了 `is_xxx_frame()` 函数用于协议检测。

## 性能特点

- **零拷贝检测**：协议检测不进行完整解析，只检查关键字节
- **早期失败**：按特征明显程度排序，快速排除不匹配的协议
- **增量解析**：返回消耗的字节数，支持流式处理

## 许可证

MIT License
