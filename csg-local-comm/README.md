# csg-local-comm

Q/CSG1209021-2019《计量自动化终端本地通信模块接口协议》解析器

## 协议概述

本库实现了中国南方电网 Q/CSG1209021-2019 标准，即**计量自动化终端本地通信模块接口协议**。该协议用于集中器/采集器与本地通信模块（电力线载波/微功率无线/以太网模块）之间的通信。

## 架构设计

遵循三层架构规范，与 DLT645-2007 和 CSG1209022 保持一致：

```
csg-local-comm/
├── src/
│   ├── link/           # 链路层
│   │   ├── address.rs  # 地址域（广播地址、节点地址）
│   │   ├── frame.rs    # 帧结构（68H...16H）
│   │   └── control.rs  # 控制字节（方向、主从、地址域标志）
│   ├── app/            # 应用层
│   │   ├── afn.rs      # 应用功能码 AFN
│   │   ├── di.rs       # 数据标识 DI（4字节，小端序）
│   │   ├── seq.rs      # 帧序列号
│   │   └── body.rs     # 应用层数据体
│   ├── engine.rs       # 顶层编解码引擎
│   ├── report.rs       # 树形渲染模块
│   └── error.rs        # 错误类型
```

## 帧格式

```
┌─────┬─────────┬─────────┬────────┬─────┬──────┬────┬─────┐
│ 68H │ L(2B)   │ C(1B)   │ [A域]  │ AFN │ SEQ  │ DI │ DATA│ CS │ 16H │
└─────┴─────────┴─────────┴────────┴─────┴──────┴────┴─────┘
  起始   长度域    控制字节  地址域   功能  序号  标识  数据  校验  结束
       (小端序)                      码
```

- **起始符**：固定 `68H`
- **长度域**：2字节小端序，表示从控制字节到校验和（不含）的总长度
- **控制字节**：包含传输方向、主从标志、地址域标志、版本、保留位
- **地址域**（可选）：根据控制字节的 D5 位决定是否存在
- **AFN**：应用功能码，1字节
- **SEQ**：帧序列号，1字节
- **DI**：数据标识，4字节（DI0 DI1 DI2 DI3，小端序传输）
- **DATA**：数据内容（可选）
- **校验和**：从起始符到数据域末尾的累加和（模256）
- **结束符**：固定 `16H`

## 控制字节格式

```
D7 D6  D5  D4 D3  D2 D1 D0
│  │   │   └──┴─  └──┴──┴── 保留（必须为000）
│  │   │     版本位（必须为00）
│  │   └──── 地址域标志（1=有地址域，0=无地址域）
│  └──────── 主从标志（1=主站，0=从站）
└─────────── 传输方向（1=下行，0=上行）
```

## 数据标识 DI

DI 为 4 字节，按**小端序**传输：`DI0 DI1 DI2 DI3`

```rust
pub struct DataIdentifier {
    pub sub_function: u8,  // DI0 - 子功能
    pub afn_match: u8,     // DI1 - 需匹配的 AFN
    pub direction: u8,     // DI2 - 传输方向标识
    pub node_role: u8,     // DI3 - 节点角色
}
```

### DI3 节点角色编码

- `00H` - 其它
- `01H` - 台区集中器
- `02H` - 公变采集器
- `04H` - 多功能户表
- ...
- **`E8H` - 集中器** ⭐（常用）
- **`EAH` - 采集器** ⭐（常用）

示例：报文中的 `02 04 02 E8` 解析为：
- DI0 = `02H`（子功能）
- DI1 = `04H`（匹配AFN=04H）
- DI2 = `02H`（方向标识）
- DI3 = `E8H`（集中器）
- 显示为：`DI=E8020402H`

## 快速开始

### 解析报文

```rust
use csg_local_comm::decode_message;

let frame_bytes = vec![
    0x68, 0x49, 0x00, 0x40, // 起始符 + 长度
    0x04, 0x11,             // AFN=04H + SEQ=17
    0x02, 0x04, 0x02, 0xE8, // DI（小端序）
    // ... 数据内容 ...
    0x56, 0x16,             // 校验和 + 结束符
];

match decode_message(&frame_bytes) {
    Ok((msg, consumed)) => {
        println!("AFN: {:02X}H - {}", msg.app.afn.to_byte(), msg.app.afn.description());
        println!("SEQ: {}", msg.app.seq);
        println!("DI: {:08X}H", msg.app.di.to_u32());
        println!("节点角色: {:?}", msg.app.di.node_role());
    }
    Err(e) => eprintln!("解析失败: {:?}", e),
}
```

### 构建报文

```rust
use csg_local_comm::{
    encode_message, Message, Frame, ControlByte, Direction,
    Afn, DataIdentifier, ApplicationLayer, ApplicationBody,
};

let msg = Message {
    frame: Frame {
        control: ControlByte::downlink_primary_without_address(),
        address: None,
        payload: vec![], // 由 encode_message 填充
    },
    app: ApplicationLayer {
        afn: Afn::DataForward,
        seq: 1,
        di: DataIdentifier::from_bytes([0x02, 0x04, 0x02, 0xE8]),
        body: ApplicationBody::DataContent(vec![0x01, 0x02, 0x03]),
    },
};

let frame_bytes = encode_message(&msg)?;
println!("生成报文: {:02X?}", frame_bytes);
```

### 协议检测

```rust
use csg_local_comm::is_csg_local_comm_frame;

let bytes = vec![0x68, 0x0C, 0x00, 0x80, ...];
if is_csg_local_comm_frame(&bytes) {
    println!("这是一个 CSG 本地通信帧");
}
```

## 支持的 AFN

| AFN  | 名称       | 说明                       |
|------|------------|----------------------------|
| 00H  | 确认/否认  | Acknowledgment / Nack      |
| 01H  | 初始化     | Initialization             |
| 02H  | 数据转发   | Data Forwarding            |
| 03H  | 参数设置   | Parameter Setting          |
| 04H  | 数据转发   | Data Forwarding (extended) |
| 05H  | 控制命令   | Control Command            |
| ...  | ...        | ...                        |

## 运行示例

### 解析真实报文

```bash
cargo run -p csg-local-comm --example parse_user_frame
```

输出：
```
【用户真实报文解析】
原始字节 (73 bytes): 68 49 00 40 04 11 02 04 02 E8 ...

✓ 解析成功，消耗 73 字节

=== 消息结构 ===
Message
├── 链路层 (Frame)
│   ├── 控制字节 (Control): 40H
│   │   ├── 传输方向: Downlink (下行)
│   │   ├── 主从标志: Primary (主站)
│   │   ├── 地址域: 无
│   │   └── 版本: 0
│   └── 地址域: 无
└── 应用层 (Application)
    ├── AFN: 04H - 数据转发
    ├── SEQ: 17
    ├── DI: E8020402H
    │   ├── DI0 (子功能): 02H
    │   ├── DI1 (AFN匹配): 04H
    │   ├── DI2 (方向): 02H
    │   └── DI3 (节点): E8H (集中器)
    └── 数据内容: [0A 18 39 36 00 19 00 50 ...]
        (60 字节)
```

### 构建报文

```bash
cargo run -p csg-local-comm --example build_frame
```

### 解析真实帧

```bash
cargo run -p csg-local-comm --example parse_real_frame
```

## 测试

```bash
# 运行所有测试
cargo test -p csg-local-comm

# 只测试库代码（不含示例）
cargo test -p csg-local-comm --lib

# 测试特定模块
cargo test -p csg-local-comm --lib address
cargo test -p csg-local-comm --lib frame
cargo test -p csg-local-comm --lib di
```

当前测试覆盖：
- ✅ 链路层：15个测试
- ✅ 应用层：31个测试
- ✅ 总计：46个测试全部通过

## 与其他协议的对比

| 特性           | CSG 本地通信 | DLT645-2007 | CSG1209022 |
|----------------|--------------|-------------|------------|
| 起始符         | 68H          | 68H (×2)    | 68H        |
| 地址域         | 可选         | 必须(6B)    | 必须(6B)   |
| 数据标识       | DI(4B)       | DI(4B)      | DA+DI      |
| 字节序         | 小端序       | 小端序      | 小端序     |
| 数据偏移       | 无           | +33H        | 无         |
| 版本位检查     | 必须为00     | 无          | 无         |

## 协议特点

1. **可选地址域**：根据控制字节的 D5 位决定是否包含地址域
2. **版本控制**：控制字节的 D4-D3 位为版本号，当前版本必须为 00
3. **严格保留位**：D2-D0 必须为 000，用于未来扩展
4. **小端序 DI**：4字节 DI 按 DI0→DI1→DI2→DI3 顺序传输
5. **节点角色编码**：DI3 用于标识设备类型（集中器E8H、采集器EAH等）

## 开发规范

- 遵循 Rust 2021 Edition
- 使用 `thiserror` 处理错误
- 使用 `proto-common` 共享字段值类型
- 集成 `spec-engine` 进行 DI 数据内容解析
- 提供树形渲染功能（`report` 模块）

## 依赖

```toml
[dependencies]
proto-common = { path = "../common", features = ["spec-engine-support"] }
spec-engine = { path = "../spec-engine" }
thiserror = "2"
```

## 许可证

MIT License

## 参考资料

- Q/CSG1209021-2019《计量自动化终端本地通信模块接口协议》
- Q/CSG1209022-2019《计量自动化终端上行通信规约》
- DL/T 645-2007《多功能电能表通信协议》
