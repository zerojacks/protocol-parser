# DL/T 645-2007 附录 A — 数据标识编码 (Data Identifier Tables)

All identifiers are `DI3 DI2 DI1 DI0`, each byte hex. Data format symbols:
`XXXXXX.XX` = measured/stored value, integer.fraction digits as shown;
`NNNNNN.NN` = a settable/parameter value; `YY`=year, `MM`=month, `WW`=weekday,
`hh`=hour, `mm`=minute, `ss`=second (all 2-digit decimal unless noted).
Unless the standard says otherwise, everything is compressed BCD, 2 digits/byte,
transmitted low-byte-first; ASCII-encoded fields are noted explicitly.

Recall: within DATA, DI2/DI1/DI0 = `FFH` (never DI3) turns a single-item
identifier into a **data-block** identifier for that dimension (see SKILL.md §7).

---

## A.1 电能量 Energy identifiers — DI3 = `00`

Format for every leaf item below: **`XXXXXX.XX` kWh/kvarh/kVAh, 4 bytes**,
top bit of the leading BCD digit is the **sign** (0=positive/正, 1=negative/负
— only meaningful for 组合有功/组合无功 combined energies), range
0.00~799999.99 (or 0.0000~79.0000 for signed combined quantities per note).

**DI2 selects the energy "type"**:

| DI2 | Energy type |
|---|---|
| 00 | 组合有功总电能 combined active total energy |
| 01 | 正向有功 forward (import) active energy |
| 02 | 反向有功 reverse (export) active energy |
| 03 | 组合无功 1 combined reactive 1 |
| 04 | 组合无功 2 combined reactive 2 |
| 05 | 第一象限无功 Q1 reactive |
| 06 | 第二象限无功 Q2 reactive |
| 07 | 第三象限无功 Q3 reactive |
| 08 | 第四象限无功 Q4 reactive |
| 09 | 正向视在 forward apparent |
| 0A | 反向视在 reverse apparent |
| 15..1E | A 相: 正向有功,反向有功,组合无功1,组合无功2,Q1,Q2,Q3,Q4,正向视在,反向视在 (A-phase, same 10-item order) |
| 29..32 | B 相 (same 10-item order as A, offset by 0x14) |
| 3D..46 | C 相 (same 10-item order as A, offset by 0x28 from A) |
| 80..86 | 关联总电能, 正向有功基波总电能, 反向有功基波总电能, 正向有功谐波总电能, 反向有功谐波总电能, 铜损有功总电能补偿量, 铁损有功总电能补偿量 |
| 94..9A | A 相: 关联/正向基波/反向基波/正向谐波/反向谐波/铜损补偿/铁损补偿 (A-phase versions; B/C phase equivalents exist at their own offsets — see full table if needed, pattern: A=94-9A, B=A8-AE, C=BC-C2) |

**DI1 selects rate/tariff (费率)**:

| DI1 | Meaning |
|---|---|
| 00 | 总 (total, all rates combined) |
| 01..3F | 费率 1 .. 费率 63 (tariff/rate 1 through 63) |
| FF | data block: total + all configured rates |

**DI0 selects the settlement period (结算日)**:

| DI0 | Meaning |
|---|---|
| 00 | (当前) current |
| 01..0C | 上 1 结算日 .. 上 12 结算日 (previous settlement day 1..12, i.e. monthly billing history) |

So e.g. `00 01 00 00` = current forward active total energy;
`00 01 05 00` = current forward active energy, tariff 5;
`00 01 00 03` = forward active total energy as of settlement-day-3-ago;
`00 01 00 FF` = data block: forward active total energy, current + all 12
settlement days; `00 01 FF 00` = data block: current forward active energy,
total + every configured rate.

---

## A.2 最大需量及发生时间 Max demand & occurrence time — DI3 = `01`

Format: **value `XX.XXXX` (kW/kvar/kVA, sign in top bit) + occurrence time
`YYMMDDhhmm`, together 8 bytes** (3 bytes value + 5 bytes time, or per table
sometimes shown as one 8-byte block).

DI2/DI1/DI0 follow the **same structure as A.1** (DI2 = quantity type using
the same 01..0A / 15-1E(A) / 29-32(B) / 3D-46(C) codes as active/reactive/
apparent — max demand doesn't track combined/harmonic/loss variants, so DI2
range is limited to 01..0A and the per-phase blocks), DI1 = rate (00=total,
01-3F=rate n, FF=block), DI0 = settlement day (00=current, 01-0C=previous
1-12, FF... not used the same way — settlement-day block uses `ZZ ZZ FF`
pattern for "current + all 12 days" per the standard's note).

E.g. `01 01 00 00` = current forward active total max demand & its
occurrence time.

---

## A.3 变量 Instantaneous variables — DI3 = `02`

| DI2 | DI1 | Item | Format | Bytes |
|---|---|---|---|---|
| 01 | 01/02/03/FF | A/B/C 相电压 phase voltage (FF=block) | XXX.X V | 2 |
| 02 | 01/02/03/FF | A/B/C 相电流 phase current (FF=block) | XXX.XXX A | 3 |
| 03 | 00/01/02/03/FF | 瞬时总/A/B/C相有功功率 instantaneous active power (sign=direction) | XX.XXXX kW | 3 |
| 04 | 00/01/02/03/FF | 瞬时总/A/B/C相无功功率 instantaneous reactive power | XX.XXXX kvar | 3 |
| 05 | 00/01/02/03/FF | 瞬时总/A/B/C相视在功率 instantaneous apparent power | XX.XXXX kVA | 3 |
| 06 | 00/01/02/03/FF | (功率因数 power factor — same pattern; sign=direction, range 0.000-1.000) | X.XXX | 2 |
| 07 | 01/02/03/FF | A/B/C 相相角 phase angle (0-360°) | XXX.X ° | 2 |
| 08 | 01/02/03/FF | A/B/C 相电压波形失真度 THD-V | XX.XX % | 2 |
| 09 | 01/02/03/FF | A/B/C 相电流波形失真度 THD-I | XX.XX % | 2 |
| 0A | 01(相)-15(次)/FF | A/B/C 相电压 n 次谐波含量 (n=1..21) | XX.XX % | 2 |
| 0B | 01(相)-15(次)/FF | A/B/C 相电流 n 次谐波含量 (n=1..21) | XX.XX % | 2 |
| 80 | 00 | 零线电流 neutral current | XXX.XXX A | 3 |
| 80 | 01 | 电网电频 grid frequency | XX.XX Hz | 2 |
| 80 | 02 | 一分钟有功平均功率 1-min avg active power | XX.XXXX kW | 3 |
| 80 | 03 | 当前有功需量 current active demand | XX.XXXX kW | 3 |
| 80 | 04 | 当前无功需量 current reactive demand | XX.XXXX kvar | 3 |
| 80 | 05 | 当前视在需量 current apparent demand | XX.XXXX kVA | 3 |
| 80 | 06 | 表内温度 meter internal temperature (top bit=sign) | XXX.X ℃ | 2 |
| 80 | 07 | 时钟电池电压(内部) clock battery voltage | XX.XX V | 2 |
| 80 | 08 | 停电抄表电池电压(外部) meter-reading battery voltage | XX.XX V | 2 |
| 80 | 09 | 内部电池工作时间 internal battery working time | XXXXXXXX 分 min | 4 |

Note: three-phase-3-wire meters map A相=Uab/Ia, B相=0, C相=Ucb/Ic.

---

## A.4 事件记录 Event records — DI3 = `03`

DI2 selects event category; DI1 usually selects phase (00=不分相/N-A,
01=A, 02=B, 03=C) or event index; DI0 selects which occurrence (00=汇总
total count/duration, 01..0A = most-recent occurrence 1..10, most recent
first).

| DI2 | Event category |
|---|---|
| 01 | 失压 loss of voltage |
| 02 | 欠压 undervoltage |
| 03 | 过压 overvoltage |
| 04 | 断相 phase loss |
| 05 | 全失压 total loss of voltage (no per-phase DI1) |
| 06 | 辅助电源失电 auxiliary power loss |
| 07 | 电压逆相序 voltage phase-reversal |
| 08 | 电流逆相序 current phase-reversal |
| 09 | 电压不平衡 voltage imbalance |
| 0A | 电流不平衡 current imbalance |
| 0B | 失流 loss of current |
| 0C | 过流 overcurrent |
| 0D | 断流 current interruption |
| 0E | 潮流反向 reverse power flow |
| 0F | 过载 overload |
| 10 | 电压合格率统计 voltage-qualification-rate monthly stats (DI1: 00=总/01-03=A/B/C相; DI0: 00=本月,01-0C=上1-12月) |
| 11 | 掉电 power-down events (no phase split) |
| 12 | 需量超限 demand-exceeded events (DI1: 01=正向有功,02=反向有功,03-06=Q1-Q4无功) |
| 30 | 编程/清零/校时/时段表/时区表/周休日/节假日/组合方式/结算日/开表盖/开端钮盒 operational logs (DI1 sub-selects which log type — see below) |

`03 30 xx` sub-types (DI1): `00`=编程记录(programming), `01`=电表清零
(meter-clear), `02`=需量清零(demand-clear), `03`=事件清零(event-clear),
`04`=校时(time-sync), `05`=时段表编程(TOU table programming), `06`=时区表编程
(time-zone table programming), `07`=周休日编程(weekly rest-day programming),
`08`=节假日编程(holiday programming), `09`=有功组合方式编程, `0A`=无功组合方式1编程,
`0B`=无功组合方式2编程, `0C`=结算日编程(settlement-day programming), `0D`=开表盖
(cover opened), `0E`=开端钮盒(terminal-block opened). DI0=`00` gives the
total occurrence count (3-byte XXXXXX); DI0=`01`..`0A` gives the detail
record for the 1st..10th most recent occurrence (payload varies by type:
typically start/end timestamp + energy deltas during the event, or
operator-code + before/after values for programming-type logs; consult the
full standard text if exact byte-for-byte layout of an event detail record
is needed — this reference captures identifier routing, not every field).

Each phase-based event's total record (`DI0=00`) = count (3 bytes, 次) +
cumulative duration (3 bytes, 分) per phase. Each detail record (`DI0=01..0A`)
generally = occurrence time + end time + energy deltas (total & per-phase,
active/reactive) recorded during the event, in 4-byte BCD kWh/kvarh chunks —
see the standard's 03 01 01 01 example structure for the canonical layout
(failure/sag record: start time[6] + end time[6] + total fwd/rev active[4
each] + combined reactive 1/2[4 each] + then per-phase energy/V/I/P/Q/PF
deltas for the affected phase).

---

## A.5 参变量 Parameters — DI3 = `04`

| DI2 | DI1 | Item | Format | Bytes |
|---|---|---|---|---|
| 00 | 01 | 日期及星期 date & weekday (0=Sunday) | YYMMDDWW | 4 |
| 00 | 02 | 时间 time | hhmmss | 3 |
| 00 | 03 | 最大需量周期 max-demand period | NN 分 min | 1 |
| 00 | 04 | 滑差时间 sliding window | NN 分 min | 1 |
| 00 | 05 | 校表脉冲宽度 calibration pulse width | XXXX ms | 2 |
| 00 | 06 | 两套时区表切换时间 dual time-zone-table switch time | YYMMDDhhmmss | 5 |
| 00 | 07 | 两套日时段表切换时间 dual day-TOU-table switch time | YYMMDDhhmmss | 5 |
| 00 | 02(2nd) | 年时区数 p≤14 / 日时段表数 q≤8 / 日时段数 m≤14 / 费率数 k≤63 / 公共假日数 n≤254 / 谐波分析次数 | 1 byte each, sub-DI0 01-06 |
| 00 | 03(2nd) | 显示相关: 自动循环显示屏数/每屏显示时间(秒)/显示电能小数位数/显示功率小数位数/按键循环显示屏数 | sub-DI0 01-05 |
| 00 | 04(3rd) | 通信地址/表号/资产管理编码(ASCII)/额定电压(ASCII)/额定电流(ASCII)/最大电流(ASCII)/有功准确度等级(ASCII)/无功准确度等级(ASCII)/电表有功常数/电表无功常数/电表型号(ASCII)/生产日期(ASCII)/协议版本号(ASCII) | sub-DI0 01-0D, sizes 6/6/32/6/6/6/4/4/3/3/10/10/16 bytes |
| 00 | 05(4th) | 电表运行状态字 1-7 (or FF=block) | 2 bytes each, sub-DI0 01-07 — see status-and-feature-words.md |
| 00 | 06 | 有功组合方式特征字/无功组合方式1特征字/无功组合方式2特征字 | 1 byte each, sub-DI0 01-03 |
| 00 | 07 | 通信速率特征字: 调试型红外/接触式红外/通信口1/通信口2/通信口3 | 1 byte each, sub-DI0 01-05 |
| 00 | 08 | 周休日特征字 / 周休日采用的日时段表号 | 1 byte each, sub-DI0 01-02 |
| 00 | 09 | 负荷记录模式字 / 冻结数据模式字 | 1 byte each, sub-DI0 01-02 |
| 00 | 0A | 负荷记录起始时间(MMDDhhmm,4B) / 第1-6类负荷记录间隔时间(NNNN分,2B each) | sub-DI0 01-07 |
| 00 | 0B | 每月第1/2/3结算日 (DDhh, 2 bytes; 9999=未设置) | sub-DI0 01-03 |
| 00 | 0C | 0级..9级密码 (4 bytes each) | sub-DI0 01-0A |
| 00 | 0D | A/B/C 相电导/电纳/电阻/电抗系数 (变压器损耗补偿系数, 2 bytes each `N.NNN`) | sub-DI0 01-0C |
| 00 | 0E | 正向/反向有功功率上限值 (XX.XXXX kW, 3B) / 电压上/下限值 (XXX.X V, 2B) | sub-DI0 01-04 |
| 01/02 | 00 | 第一/二套时区表: 第1-14时区起始日期(MMDD)及时段表号(NN), 3 bytes each | DI0 01-0E |
| 01/02 | 00 sub | 第一/二套第1-8日时段表: 每日最多14个时段, 每段起始时间(hhmm)+费率号(NN), 3 bytes each | DI1 01-08 |
| 03 | 00 | 第1-254公共假日日期及日时段表号 (YYMMDDNN, 4B) | DI0 01-FE |
| 03 | 01/02 | 自动/按键循环显示第1-254屏显示数据项 (4-byte DI code being displayed) | DI0 01-FE |
| 80 | 00 | 厂家软件版本号/厂家硬件版本号/厂家编号 (ASCII, manufacturer-specific) | sub-DI0 01-03, 4 bytes each |

---

## A.6 冻结量 Freeze data — DI3 = `05`

DI2 selects freeze trigger type:

| DI2 | Trigger |
|---|---|
| 00 | 定时冻结 timed/periodic freeze |
| 01 | 瞬时冻结 instantaneous freeze (on-demand) |
| 02 | 两套时区表切换 dual time-zone-table switch |
| 03 | 两套日时段表切换 dual day-TOU-table switch |

DI1 selects the data category within that freeze snapshot (mirrors A.1-A.3
layout): `00`=冻结时间(YYMMDDhhmm,5B), `01`=正向有功电能(4×n B), `02`=反向有功
电能, `03`=无功组合1电能, `04`=无功组合2电能, `05..08`=Q1-Q4无功电能, `09`=正向
有功最大需量及时间(8×n B), `0A`=反向有功最大需量及时间, `10`=变量数据(总/ABC相
有功/无功功率, 3×8 B), `FF`=整个冻结数据块. ("n" = actual configured rate
count + 1 for the total.)

DI0 = which occurrence, `01`..`0C` = most-recent 1st..12th freeze event (for
定时冻结) or 1st..3rd (for 瞬时冻结, per the standard's example) — most recent
occurrences are numbered starting at 01.

---

## A.7 负荷记录 Load profile — DI3 = `06`

DI2 = which load-profile channel: `00`=总加(aggregate), `01`..`06` = 第1-6类
负荷 (load classes 1-6, per the modes configured in 负荷记录模式字).

DI1/DI0 (as sent by the **master**, request format only):
- `00` = 最早记录块 earliest stored record block (1 byte payload, arbitrary/ignored)
- `01` = 给定时间记录块 record block at/after a given time — request payload is `YYMMDDhhmmNN` (6 bytes: given start time + N = number of blocks wanted)
- `02` = 最近一个记录块 most recent record block (1 byte payload)

The **slave's response** (uploaded load-profile data) uses a different,
fixed binary structure — NOT the DI/BCD-per-field convention used elsewhere.
See "Appendix B — Load Profile Record Format" below.

### Appendix B: 负荷记录 upload record format

```
A0H A0H          负荷记录起始码 (or E0H E0H = block marked bad/incorrect), 2 bytes
<len>            负荷记录字节数, 1 byte (hex byte count of what follows, up to the end code)
YY MM DD hh mm   负荷记录存储时间, 5 bytes
<17 bytes>       电压、电流、频率: Ua,Ub,Uc (2B each, 0.1V) + Ia,Ib,Ic (3B each, 0.001A) + freq (2B, 0.01Hz)
AAH              块分隔码 (block separator)
<24 bytes>       有、无功功率: 总/A/B/C 有功功率 (3B each, 0.0001kW) + 总/A/B/C 无功功率 (3B each, 0.0001kvar)
AAH              块分隔码
<8 bytes>        功率因数: 总/A/B/C (2B each, 0.001)
AAH              块分隔码
<16 bytes>       有、无功总电能: 正向有功(4B,0.01kWh) + 反向有功(4B) + 组合无功1(4B,0.01kvarh) + 组合无功2(4B)
AAH              块分隔码
<6 bytes>        当前需量: 有功需量(3B,0.0001kW) + 无功需量(3B,0.0001kvar)
AAH              块分隔码
<1 byte>         负荷记录累加校验码 (sum, mod 256, from the first A0H through the last block's end)
E5H              负荷记录结束码
```

If a data category is disabled in 负荷记录模式字 (see status-and-feature-words.md),
that whole block is empty — just the `AAH` separator with nothing before it.
