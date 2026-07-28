# DL/T 645-2007 附录 C — 状态字、特征字、模式字、错误信息字

All words below are bitfields. Bit0 = LSB. "保留" = reserved (ignore/zero).

## 电表运行状态字 1 (Meter status word 1) — 2 bytes, DI `04 00 05 01`

| Bit | Meaning |
|---|---|
| 0 | 保留 |
| 1 | 需量积算方式: 0=滑差(sliding), 1=区间(fixed block) |
| 2 | 时钟电池: 0=正常, 1=欠压 |
| 3 | 停电抄表电池: 0=正常, 1=欠压 |
| 4 | 有功功率方向: 0=正向, 1=反向 |
| 5 | 无功功率方向: 0=正向, 1=反向 |
| 6-15 | 保留 |

## 电表运行状态字 2 — DI `04 00 05 02`

Bits 0-2 = A/B/C 相有功功率方向 (0=正向,1=反向); bit 3 保留; bits 4-6 = A/B/C
相无功功率方向; bit 7 保留; bits 8-15 保留.

## 电表运行状态字 3 (操作类 / operational) — DI `04 00 05 03`

| Bit | Meaning |
|---|---|
| 0 | 当前运行时段: 0=第一套, 1=第二套 |
| 1-2 | 供电方式: 00=主电源, 01=辅助电源, 10=电池供电 |
| 3 | 编程允许 (编程按键状态): 0=禁止, 1=许可 |
| 4 | 继电器状态: 0=通, 1=断 |
| 5-15 | 保留 |

## 电表运行状态字 4/5/6 (A/B/C 相故障状态) — DI `04 00 05 04/05/06`

Same bit layout for each phase (0 = no fault, 1 = fault active):

| Bit | Fault |
|---|---|
| 0 | 失压 loss of voltage |
| 1 | 欠压 undervoltage |
| 2 | 过压 overvoltage |
| 3 | 失流 loss of current |
| 4 | 过流 overcurrent |
| 5 | 过载 overload |
| 6 | 潮流反向 reverse power flow |
| 7 | 断相 phase loss |
| 8-15 | 保留 |

## 电表运行状态字 7 (合相故障状态 / combined-phase faults) — DI `04 00 05 07`

| Bit | Fault |
|---|---|
| 0 | 电压逆相序 |
| 1 | 电流逆相序 |
| 2 | 电压不平衡 |
| 3 | 电流不平衡 |
| 4 | 辅助电源失电 |
| 5 | 掉电 |
| 6 | 需量超限 |
| 7 | 保留 |
| 8-15 | 保留 |

## 有功组合方式特征字 — DI `04 00 06 01`

| Bit | Meaning |
|---|---|
| 0 | 正向有功 (0 不加, 1 加) |
| 1 | 正向有功 (0 不减, 1 减) |
| 2 | 反向有功 (0 不加, 1 加) |
| 3 | 反向有功 (0 不减, 1 减) |
| 4-7 | 保留 |

## 无功组合方式特征字 (无功组合1/2, 结构相同) — DI `04 00 06 02/03`

| Bit | Meaning |
|---|---|
| 0 | Ⅰ象限 (0 不加, 1 加) |
| 1 | Ⅰ象限 (0 不减, 1 减) |
| 2 | Ⅱ象限 (0 不加, 1 加) |
| 3 | Ⅱ象限 (0 不减, 1 减) |
| 4 | Ⅲ象限 (0 不加, 1 加) |
| 5 | Ⅲ象限 (0 不减, 1 减) |
| 6 | Ⅳ象限 (0 不加, 1 加) |
| 7 | Ⅳ象限 (0 不减, 1 减) |

## 周休日特征字 — DI `04 00 08 01`

Bit0=周日(Sunday) ... bit6=周六(Saturday); bit7 保留. 0=休息(rest day), 1=工作(work day).

## 通信速率特征字 (调试型/接触式/通信口1/2/3 各1字节) — DI `04 00 07 01..05`

| Bit | Rate |
|---|---|
| 0 | 保留 |
| 1 | 600 bps |
| 2 | 1200 bps |
| 3 | 2400 bps |
| 4 | 4800 bps |
| 5 | 9600 bps |
| 6 | 19200 bps |
| 7 | 保留 |

Only one bit may be set at a time when used as a "change baud rate" request
(§7.8 of the standard, control code 17H) — bits 0-7 correspond one-to-one to
a single rate; combining bits is invalid.

## 负荷记录模式字 — DI `04 00 09 01`

| Bit | Data category recorded (1=recorded, 0=not) |
|---|---|
| 0 | 电压、电流、频率 |
| 1 | 有、无功功率 |
| 2 | 功率因数 |
| 3 | 有、无功总电能 |
| 4 | 四象限无功总电能 |
| 5 | 当前需量 |
| 6-7 | 保留 |

## 冻结数据模式字 — DI `04 00 09 02`

| Bit | Data category recorded (1=recorded, 0=not) |
|---|---|
| 0 | 正向有功电能 |
| 1 | 反向有功电能 |
| 2 | 组合无功 1 电能 |
| 3 | 组合无功 2 电能 |
| 4 | 四象限无功总电能 |
| 5 | 正向有功最大需量及发生时间 |
| 6 | 反向有功最大需量及发生时间 |
| 7 | 变量 |

## 错误信息字 ERR (Error word) — always 1 byte, sent as DATA when control code has D6=1 (error response, e.g. D1H/D2H/D4H/D6H/D7H/D8H/D9H/DAH/DBH)

| Bit | Meaning |
|---|---|
| 0 | 其他错误 other error |
| 1 | 无请求数据 requested data does not exist |
| 2 | 密码错/未授权 wrong password / not authorized |
| 3 | 通讯速率不能更改 comm. rate cannot be changed |
| 4 | 年时区数超 year time-zone count exceeded |
| 5 | 日时段数超 day-TOU-segment count exceeded |
| 6 | 费率数超 rate/tariff count exceeded |
| 7 | 保留 |

Multiple bits may be set simultaneously if more than one error condition applies.
