# proto-common

多个电力计量通信协议解析器（`csg1209022`、`dlt645-2007` 等）共享的**极薄**工具函数库。

只放"不同协议之间确实通用、且不带协议语义"的东西：

- `checksum`：帧校验和（八位位组算术和，不考虑进位）
- `bcd`：BCD 编解码（时间域、地址域等帧层字段用，和 `spec-engine` 内容层的 BCD 解码互相独立）
- `field_value`（feature `spec-engine-support`）：报文展示树用的值类型，与 `spec_engine::Value` 结构镜像但类型独立

## `field_value` 模块

### 为什么需要独立的 `FieldValue` 类型？

`spec_engine` 是独立发布、独立演进的外部依赖。如果本 crate 的公开 API（报文展示树）
直接使用 `spec_engine::Value`，那么 spec-engine 每次改动都可能是本 crate 的
破坏性变更（breaking change），版本号也被迫跟着它的节奏走。

所以这里定义一份**结构上镜像、但完全独立**的 `FieldValue`，只在 DI 内容解析
结果要嵌入展示树的边界点做一次转换（`from_spec_engine` 函数）。以后 spec-engine
的 `Value` 变了，只需要改这一个函数；本 crate 的公开类型和依赖它的下游代码都不受影响。

### 使用方式

启用 `spec-engine-support` feature：

```toml
[dependencies]
proto-common = { path = "../common", features = ["spec-engine-support"] }
```

然后在代码中：

```rust
use proto_common::{FieldValue, from_spec_engine};

// 将 spec_engine::Value 转换为 FieldValue
let spec_value = spec_engine::parse(...);
let field_value = from_spec_engine(&spec_value);
```

**原则**：不要往这里加"通用 Frame trait"、"通用应用层结构"之类的抽象。等真的出现第三个协议、
确认某段逻辑在多个协议里逐字节一致时，再从对应协议 crate 里下沉过来。
