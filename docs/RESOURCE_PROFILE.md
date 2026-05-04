# VoidPointer 资源依据与硬件/协议 Profile

本文档把 `resources/` 中已经存在的芯片手册和例程转化为固件实现时可直接引用的参数来源与目标 profile。它不是 WebHID 协议文档，也不替代 `CONFIG_SPEC.md`。

---

## 1. 资源索引

### 1.1 IMU / LSM6DSV

| 资源 | 用途 |
| --- | --- |
| `resources/main/LSM6DSVTR.PDF` | LSM6DSV datasheet，SFLP、FIFO、ODR、wake/motion、寄存器定义。 |
| `resources/main/SCH_Schematic_LSM6DSV_VR.pdf` | LSM6DSV 参考原理图。 |
| `platform/APP/lsm6dsv.c` | 当前工程已有 LSM6DSV 初始化与 SFLP FIFO 读取原型。 |
| `platform/APP/include/lsm6dsv.h` | 当前工程已有寄存器地址、WHO_AM_I、FIFO tag 定义。 |

### 1.2 I2C

| 资源 | 用途 |
| --- | --- |
| `resources/main/CH585/EXAM/I2C/src/app_i2c.c` | WCH I2C interrupt master/slave 状态机例程。 |
| `resources/main/CH585/EXAM/I2C/src/app_i2c.h` | I2C API/状态/错误码例程。 |
| `resources/main/CH585/EXAM/I2C/src/Main.c` | I2C polling 与 interrupt 使用方式示例。 |

### 1.3 BLE HID Mouse

| 资源 | 用途 |
| --- | --- |
| `resources/main/CH585/EXAM/BLE/HID_Mouse/Profile/hidmouseservice.c` | BLE HID mouse service、report map、GATT attribute 表。 |
| `resources/main/CH585/EXAM/BLE/HID_Mouse/Profile/include/hidmouseservice.h` | BLE HID report id、feature flags。 |
| `resources/main/CH585/EXAM/BLE/HID_Mouse/APP/hidmouse.c` | BLE HID mouse app、GAP 参数、HidDev_Report 使用方式。 |
| `resources/main/CH585/EXAM/BLE/HID_Mouse/APP/hidmouse_main.c` | BLE HID mouse 初始化流程。 |
| `platform/Profile/hidmouseservice.c` | 当前工程拷贝/改造后的 HID mouse service。 |
| `platform/APP/ble_hid_app.c` | 当前工程 BLE HID app / GAP / advertising glue。 |

### 1.4 USB HID / Vendor HID

| 资源 | 用途 |
| --- | --- |
| `resources/main/CH585/EXAM/USB/Device/HID_CompliantDev/src/Main.c` | USB FS compatible HID device 例程，含 vendor HID report descriptor 与控制传输处理。 |
| `resources/main/CH585/EXAM/USB/USBHS/DEVICE/CompatibilityHID/User/usb_desc.c` | USBHS vendor HID descriptor，含 FS/HS report descriptor。 |
| `resources/main/CH585/EXAM/USB/USBHS/DEVICE/CompatibilityHID/User/usbd_compatibility_hid.c` | USBHS HID 数据收发、端点 busy 处理示例。 |

---

## 2. IMU 基础事实

### 2.1 LSM6DSV identity

| 项 | 值 |
| --- | --- |
| I2C 7-bit address | `0x6A`，以实际 SA0 硬件为准。 |
| 当前工程 8-bit address define | `(0x6A << 1)`。 |
| `WHO_AM_I` register | `0x0F`。 |
| `WHO_AM_I` expected value | `0x70`。 |

### 2.2 SFLP game rotation

从 `LSM6DSVTR.PDF`：

- LSM6DSV 内置 Sensor Fusion Low-Power，简称 SFLP。
- SFLP 可输出 6-axis game rotation vector。
- Game rotation vector 是 quaternion 姿态表示。
- FIFO 中存储 X/Y/Z quaternion components。
- Rust 侧根据 `x/y/z` 计算 `w = sqrt(1 - x² - y² - z²)`。

当前工程已经按该方向实现：

- C 读取 raw half-float `x/y/z`。
- Rust `AttitudeData::from(sflp_game_rotation_raw_t)` 转换为 quaternion + roll/pitch/yaw。

### 2.3 SFLP ODR

`SFLP_ODR (5Eh)` 中 `SFLP_GAME_ODR_[2:0]` 定义：

| bits | ODR |
| --- | --- |
| `000` | 15 Hz |
| `001` | 30 Hz |
| `010` | 60 Hz |
| `011` | 120 Hz，default |
| `100` | 240 Hz |
| `101` | 480 Hz |

目标 profile 建议：

| Profile | SFLP ODR | 说明 |
| --- | --- | --- |
| `Active` | 120 Hz 起步，允许试验 240 Hz | 空鼠按住期间优先低延迟。当前原型 `SFLP_ODR = 0x5B`，低 3 bit 对应 120 Hz。 |
| `Suspend` | 15/30 Hz 或关闭 SFLP，仅保留运动检测 | 有连接静置，优先低功耗与秒唤醒。 |
| `Sleep` | 关闭 SFLP，仅保留 wake/motion interrupt | 无连接静置，RF 关闭，极低功耗唤醒。 |

### 2.4 FIFO word 与 tag

从 `LSM6DSVTR.PDF`：

- FIFO 每个 word 为 7 bytes。
- 第 1 byte 是 `FIFO_DATA_OUT_TAG (78h)`。
- 后 6 bytes 是 X/Y/Z 三轴数据。
- FIFO sample count 来自 `FIFO_STATUS1 (1Bh)` 与 `FIFO_STATUS2 (1Ch)` 中的 `DIFF_FIFO_[8:0]`。
- `FIFO_DATA_OUT_TAG` 中 `TAG_SENSOR_[4:0]` 标识 sensor。
- SFLP game rotation vector tag 为 `0x13`。

当前工程定义：

| define | 值 |
| --- | --- |
| `LSM6DSV_REG_FIFO_STATUS1` | `0x1B` |
| `LSM6DSV_REG_FIFO_STATUS2` | `0x1C` |
| `LSM6DSV_REG_FIFO_DATA_OUT_TAG` | `0x78` |
| `LSM6DSV_FIFO_TAG_SFLP_GAME` | `0x13` |

目标实现：

- C 层 FIFO parser 只筛选 `TAG_SENSOR == 0x13` 的 SFLP sample。
- C 层不做姿态解算。
- C 层将最新 raw x/y/z 回调 Rust。
- Rust 使用 latest-sample cache。
- FIFO 多样本时优先低延迟，可丢弃旧 sample。

### 2.5 SFLP enable 相关寄存器

当前工程中已有寄存器定义：

| Register | Address | 说明 |
| --- | --- | --- |
| `FUNC_CFG_ACCESS` | `0x01` | embedded function register access。 |
| `EMB_FUNC_EN_A` | `0x04` | embedded functions enable。 |
| `EMB_FUNC_FIFO_EN_A` | `0x44` | embedded function FIFO enable。 |
| `SFLP_ODR` | `0x5E` | SFLP ODR。 |
| `EMB_FUNC_INIT_A` | `0x66` | embedded function init。 |
| `FIFO_CTRL4` | `0x0A` | FIFO mode。 |

从 datasheet：

- `EMB_FUNC_EN_A.SFLP_GAME_EN = 1` 使能 SFLP game rotation。
- `EMB_FUNC_INIT_A.SFLP_GAME_INIT = 1` 可重新初始化 SFLP game。
- embedded function registers 需要通过 `FUNC_CFG_ACCESS.EMB_FUNC_REG_ACCESS` 访问。

### 2.6 当前原型初始化序列

当前 `platform/APP/lsm6dsv.c` 原型执行：

1. `CTRL3 = 0x01` soft reset。
2. delay。
3. check `WHO_AM_I == 0x70`。
4. `CTRL3 = 0x44`。
5. `CTRL8 = 0x00`。
6. `CTRL6 = 0x04`。
7. `CTRL1 = 0x06`。
8. `CTRL2 = 0x06`。
9. `FUNCTIONS_ENABLE = 0x40`。
10. `FUNC_CFG_ACCESS = 0x80`。
11. `EMB_FUNC_EN_A = 0x02`。
12. `EMB_FUNC_FIFO_EN_A = 0x02`。
13. `SFLP_ODR = 0x5B`。
14. `EMB_FUNC_INIT_A = 0x02`。
15. `FUNC_CFG_ACCESS = 0x00`。
16. `FIFO_CTRL4 = 0x06`。

目标实现不应把这组值直接散落在代码里，而应整理为 profile table：

- `imu_profile_active[]`
- `imu_profile_suspend[]`
- `imu_profile_sleep[]`

每条 profile entry 包含：

- register bank。
- address。
- value。
- delay requirement。
- 是否必须在 power-down 下修改。

---

## 3. IMU Profile 建议

v1 profile 决策：

- `Active` profile 先固定默认起点，用于空鼠体验和 BLE/USB HID 闭环。
- `Suspend` / `Sleep` profile 标为 tuning required；v1 固定 profile 目标、接口和寄存器表结构，但具体 ODR、threshold、duration 需按误触发、唤醒延迟和功耗实测调优。

### 3.1 Active profile

用途：Action/Middle 空鼠期间。

建议起步参数：

| 项 | 值 |
| --- | --- |
| SFLP | enabled |
| SFLP ODR | 120 Hz 起步，实测可试 240 Hz |
| FIFO mode | continuous |
| FIFO sample policy | latest sample |
| INT | FIFO data/watermark 或 embedded data ready，按实际调试选择 |
| Rust 行为 | motion active/arming 时请求 FIFO async read |

实现依据：

- 当前原型已能读取 SFLP FIFO。
- HID mouse report 目标可高于 IMU ODR，report 层通过速度积分和 pending 实现平滑。

### 3.2 Suspend profile

状态：tuning required。以下参数是起点，不是最终拍板值。

用途：有 BLE/2.4G 连接但静置。

建议起步参数：

| 项 | 值 |
| --- | --- |
| SFLP | 可关闭，除非需要姿态预热 |
| Accel | low-power mode |
| Gyro | sleep/power-down，按 wake 延迟实测 |
| Motion interrupt | enabled |
| ODR | 15/30 Hz 级别 |
| RF | keep connected |

Rust 策略：

- 有连接且无输入/HID pending，超过 suspend timeout 后进入。
- IMU wake 或按键/编码器唤醒后回到 Active。

### 3.3 Sleep profile

状态：tuning required。以下参数是起点，不是最终拍板值。

用途：无连接静置，RF 关闭。

建议起步参数：

| 项 | 值 |
| --- | --- |
| SFLP | disabled |
| Accel | ultra-low-power motion/wake source |
| Gyro | power-down |
| RF | off |
| Wake source | GPIO buttons, encoder, USB attach, IMU motion |

Rust 策略：

- 无连接、无 HID pending、无配置写入、laser off，且从无线断连时刻计算超过 `disconnect_sleep_timeout_ms` 后进入。
- wake 后先进入 Active，再由 `vp_core_poll()` 根据连接状态决定是否回 Suspend 或 Sleep。

---

## 4. IMU Wake Strategy

### 4.1 结论

默认唤醒不需要依赖 SFLP 角度检测。

原因：

- SFLP game rotation 需要 accel + gyro，用于 Active 下姿态/空鼠控制更合适。
- Suspend/Sleep 唤醒只需要判断“设备被拿起/移动/震动/切换状态”，不需要完整姿态角。
- LSM6DSV 本身提供基于 accelerometer 的 wake-up、activity/inactivity、stationary/motion、significant motion 等硬件中断，功耗更低，更符合唤醒需求。

推荐：

| 场景 | 默认唤醒方案 |
| --- | --- |
| `Active` | IMU FIFO/SFLP INT + GPIO input。 |
| `Suspend` | activity/inactivity 或 wake-up interrupt + GPIO input，RF 保持连接。 |
| `Sleep` | wake-up interrupt 或 significant motion + GPIO input + USB attach，RF 关闭。 |

SFLP 只在以下场景使用：

- Action/Middle motion active。
- motion arming 时捕获基准姿态。
- 唤醒后需要快速进入空鼠模式时作为姿态源。

### 4.2 可用 IMU 唤醒能力

`LSM6DSVTR.PDF` 明确支持：

- wake-up events。
- activity/inactivity。
- stationary/motion detection。
- significant motion detection。
- tilt detection。
- FSM programmable motion pattern。

### 4.3 Basic interrupt enable

相关寄存器：

| Register | Address | 说明 |
| --- | --- | --- |
| `FUNCTIONS_ENABLE` | `0x50` | basic interrupt 与 activity/inactivity enable。 |
| `INACTIVITY_DUR` | `0x54` | activity/inactivity 配置。 |
| `INACTIVITY_THS` | `0x55` | activity/inactivity threshold。 |
| `WAKE_UP_THS` | `0x5B` | wake-up threshold。 |
| `WAKE_UP_DUR` | `0x5C` | wake-up duration 与 sleep duration。 |
| `MD1_CFG` | `0x5E` | basic interrupt route to INT1。 |
| `MD2_CFG` | `0x5F` | basic interrupt route to INT2。 |
| `WAKE_UP_SRC` | `0x45` | wake-up/activity status source。 |

`FUNCTIONS_ENABLE.INTERRUPTS_ENABLE` 需要置位以启用 basic interrupts。

`FUNCTIONS_ENABLE.INACT_EN_[1:0]` 可配置 activity/inactivity 行为：

| bits | 行为 |
| --- | --- |
| `00` | 只产生 stationary/motion interrupt，不改变 accel/gyro 配置。 |
| `01` | inactivity 后 accel 切 low-power mode 1，gyro 不变。 |
| `10` | inactivity 后 accel 切 low-power mode 1，gyro sleep。 |
| `11` | inactivity 后 accel 切 low-power mode 1，gyro power-down。 |

推荐：

| Profile | `INACT_EN` 建议 |
| --- | --- |
| `Suspend` | `10` 起步，必要时试 `11`。 |
| `Sleep` | `11`。 |

### 4.4 Activity/Inactivity 参数

`INACTIVITY_DUR (54h)` 关键字段：

| 字段 | 说明 |
| --- | --- |
| `SLEEP_STATUS_ON_INT` | 选择 INT pin 输出 sleep status 或 sleep change。 |
| `WU_INACT_THS_W_[2:0]` | wake-up/activity threshold LSB 权重。 |
| `XL_INACT_ODR_[1:0]` | inactivity 下 accel ODR。 |
| `INACT_DUR_[1:0]` | stationary→motion 转换需要的连续过阈值次数。 |

`XL_INACT_ODR_[1:0]`：

| bits | ODR |
| --- | --- |
| `00` | 1.875 Hz |
| `01` | 15 Hz |
| `10` | 30 Hz |
| `11` | 60 Hz |

`WU_INACT_THS_W_[2:0]` threshold LSB 权重：

| bits | mg/LSB |
| --- | --- |
| `000` | 7.8125 mg |
| `001` | 15.625 mg |
| `010` | 31.25 mg |
| `011` | 62.5 mg |
| `100` | 125 mg |
| `101`/`110`/`111` | 250 mg |

### 4.5 Wake-up 参数

`WAKE_UP_THS (5Bh)`：

| 字段 | 说明 |
| --- | --- |
| `WK_THS_[5:0]` | wake-up threshold，分辨率由 `WU_INACT_THS_W_[2:0]` 决定。 |
| `USR_OFF_ON_WU` | 是否使用带用户 offset 的低通数据进入 wake/activity 检测。 |

`WAKE_UP_DUR (5Ch)`：

| 字段 | 说明 |
| --- | --- |
| `WAKE_DUR_[1:0]` | wake-up duration，1 LSB = 1/ODR_XL。 |
| `SLEEP_DUR_[3:0]` | sleep mode duration，1 LSB = 512/ODR_XL。 |

### 4.6 INT 路由

可用路由：

| Register | Field | 说明 |
| --- | --- | --- |
| `MD1_CFG.INT1_SLEEP_CHANGE` | route activity/inactivity to INT1。 |
| `MD1_CFG.INT1_WU` | route wake-up event to INT1。 |
| `MD1_CFG.INT1_EMB_FUNC` | route embedded function event to INT1。 |
| `MD2_CFG.INT2_SLEEP_CHANGE` | route activity/inactivity to INT2。 |
| `MD2_CFG.INT2_WU` | route wake-up event to INT2。 |
| `MD2_CFG.INT2_EMB_FUNC` | route embedded function event to INT2。 |

最终使用 INT1 还是 INT2 以 PCB 连接为准。

### 4.7 推荐唤醒方案

#### Suspend

目标：保持 BLE/2.4G 连接，拿起秒醒。

建议：

- RF 保持连接。
- SFLP 默认关闭或降到最低，仅当需要姿态预热时保留。
- 使用 activity/inactivity 或 wake-up interrupt。
- accel low-power ODR 起步 15 Hz。
- gyro sleep 起步。
- INT route 到 MCU wake GPIO。

推荐起点：

| 参数 | 起点 |
| --- | --- |
| `XL_INACT_ODR` | 15 Hz |
| `INACT_EN` | `10` |
| wake threshold | 先按 31.25 mg/LSB 权重试配 |
| wake duration | 1~2 sample 起步 |

#### Sleep

目标：无连接静置，最低功耗。

建议：

- RF off。
- SFLP disabled。
- gyro power-down。
- accel low-power motion/wake。
- 使用 wake-up interrupt 或 significant motion。
- GPIO buttons / encoder / USB attach 同时作为 wake source。

推荐起点：

| 参数 | 起点 |
| --- | --- |
| `XL_INACT_ODR` | 1.875 Hz 或 15 Hz，根据唤醒延迟实测 |
| `INACT_EN` | `11` |
| wake threshold | 比 Suspend 略高，避免桌面微振误唤醒 |
| wake duration | 1~4 sample 实测 |

### 4.8 SFLP 角度检测的定位

SFLP 角度检测不作为默认唤醒方案。

可以保留为可选策略：

- 如果后续发现 wake-up/activity 太敏感或太迟钝，可在 `Suspend` 中低频保留 SFLP，以姿态变化角度作为二级确认。
- 该策略会增加功耗，因为 SFLP game rotation 依赖 accel + gyro。
- 默认不启用。

### 4.9 CH585 GPIO/EXTI 依据

来自 `resources/main/CH585/EXAM/SRC/StdPeriphDriver/inc/CH58x_gpio.h`：

`GPIOITModeTpDef` 支持：

| WCH enum | 含义 |
| --- | --- |
| `GPIO_ITMode_LowLevel` | 低电平触发。 |
| `GPIO_ITMode_HighLevel` | 高电平触发。 |
| `GPIO_ITMode_FallEdge` | 下降沿触发。 |
| `GPIO_ITMode_RiseEdge` | 上升沿触发。 |

来自 `resources/main/CH585/EXAM/SRC/StdPeriphDriver/CH58x_gpio.c`：

- `GPIOA_ITModeCfg(pin, mode)` / `GPIOB_ITModeCfg(pin, mode)` 根据 `mode` 配置 level/edge 和触发极性。
- WCH 标准外设 API 没有直接提供 both-edge 模式。
- VoidPointer 目标 FFI `vp_exti_edge_t` 初版只暴露 `Rising` / `Falling` / `Both`，其中 `Both` 需要平台层通过切换边沿或读取电平策略模拟；如果实测不可靠，应限制具体输入只使用单边沿 + timer/debounce 策略。
- 当前普通按键方案本来就是单边沿 EXTI 唤醒 + debounce timer，因此不依赖硬件 both-edge。
- 编码器 A/B 需要任意边沿；平台层需验证 CH585 上 both-edge 模拟策略是否满足高速滚动，必要时使用更底层寄存器策略或定时采样兜底。

---

## 5. HID Mouse Profile

### 5.1 BLE HID Mouse 例程依据

来自 `resources/main/CH585/EXAM/BLE/HID_Mouse/Profile/hidmouseservice.c`：

HID report map 定义：

- Usage Page：Generic Desktop。
- Usage：Mouse。
- Buttons：Button 1..3。
- Button bit count：8 bits。
- X/Y/Wheel：3 个 8-bit relative axis。
- Logical min/max：`-127..127`。

目标 mouse report：

| byte | 字段 | 说明 |
| --- | --- | --- |
| 0 | buttons | bit0 left, bit1 right, bit2 middle，其余保留。 |
| 1 | dx | signed i8，范围建议 `-127..127`。 |
| 2 | dy | signed i8，范围建议 `-127..127`。 |
| 3 | wheel | signed i8，范围建议 `-127..127`。 |

来自 `resources/main/CH585/EXAM/BLE/HID_Mouse/Profile/include/hidmouseservice.h`：

| 项 | 值 |
| --- | --- |
| `HID_NUM_REPORTS` | `3` |
| `HID_RPT_ID_MOUSE_IN` | `0` |
| `HID_RPT_ID_FEATURE` | `0` |
| `HID_FEATURE_FLAGS` | `HID_FLAGS_REMOTE_WAKE` |

来自 `resources/main/CH585/EXAM/BLE/HID_Mouse/APP/hidmouse.c`：

| 项 | 值 |
| --- | --- |
| input report length | `4` bytes |
| HID send API | `HidDev_Report(HID_RPT_ID_MOUSE_IN, HID_REPORT_TYPE_INPUT, len, buf)` |
| min conn interval | `8 * 1.25 ms = 10 ms` |
| max conn interval | `8 * 1.25 ms = 10 ms` |
| slave latency | `0` |
| supervision timeout | `500 * 10 ms = 5 s` |
| bonding | enabled |
| IO capability | no input/no output |

目标实现：

- Rust 生成 4-byte mouse report。
- C BLE glue 只调用 `HidDev_Report()`。
- C 不解释 buttons/dx/dy/wheel。
- Rust report 层避免发送 `-128`。
- Route disconnected 时 Rust 清 motion pending，必要时保留 release safety。

### 5.2 USB HID / Vendor HID 例程依据

来自 `resources/main/CH585/EXAM/USB/Device/HID_CompliantDev/src/Main.c`：

- FS vendor-defined HID example。
- Report descriptor 使用 Usage Page `0xFF00`。
- Input/Output report count `0x40`，即 64 bytes。
- Endpoint 1 IN/OUT，interrupt transfer。

来自 `resources/main/CH585/EXAM/USB/USBHS/DEVICE/CompatibilityHID/User/usb_desc.c`：

- HS/FS vendor HID descriptor 示例。
- FS endpoint max packet size：64 bytes。
- HS endpoint max packet size：512 bytes。
- bInterval：1 ms。
- Vendor HID report descriptor 使用 Usage Page `0xFF00`。

目标实现：

- 当前硬件使用 USBHS 口，USB vendor/WebHID 首选按 HS 能力实现。
- USBHS physical report payload 可使用 512 bytes。
- USB FS fallback 为 64 bytes。
- BLE/2.4G vendor payload 按各自 route MTU 或私有协议能力分片。
- 逻辑 Vendor/WebHID 协议在 `VENDOR_PROTOCOL.md` 中定义，不假设所有 route 物理包大小一致。
- USB mouse HID 与 USB vendor/WebHID 可分 interface 或后续 composite 设计。
- USB mouse report 复用 BLE mouse 4-byte report 语义。
- USB descriptor 具体 composite 结构放到 WebHID/USB 协议文档中定稿。

---

## 6. I2C Profile

来自 `resources/main/CH585/EXAM/I2C`：

- CH585 I2C 可配置 400 kHz。
- GPIO 需配置上拉输入。
- WCH 例程提供 interrupt state machine，可作为 async FIFO 读取基础。

目标实现：

| 项 | 值 |
| --- | --- |
| I2C role | master |
| speed | 400 kHz |
| ACK | enabled |
| address mode | 7-bit |
| SDA/SCL | 以 PCB 为准，当前工程规划 `PB12/PB13` |
| blocking read | 仅初始化/诊断允许，motion 主路径禁用 |
| async read | IMU FIFO 主路径 |

---

## 7. 需要写入代码的常量来源

### 7.1 HID constants

| 常量 | 值 | 来源 |
| --- | --- | --- |
| mouse report len | `4` | BLE HID Mouse example |
| buttons byte | byte 0 | BLE HID report map |
| dx byte | byte 1 | BLE HID report map |
| dy byte | byte 2 | BLE HID report map |
| wheel byte | byte 3 | BLE HID report map |
| HID logical range | `-127..127` | BLE HID report map |
| vendor HID FS payload | `64` bytes | USB HID examples |
| vendor HID HS payload | `512` bytes | USBHS CompatibilityHID example |

### 7.2 IMU constants

| 常量 | 值 | 来源 |
| --- | --- | --- |
| LSM6DSV WHO_AM_I | `0x70` | datasheet/current driver |
| SFLP FIFO tag | `0x13` | datasheet/current driver |
| FIFO word bytes | `7` | datasheet |
| SFLP ODR 15 Hz | bits `000` | datasheet |
| SFLP ODR 30 Hz | bits `001` | datasheet |
| SFLP ODR 60 Hz | bits `010` | datasheet |
| SFLP ODR 120 Hz | bits `011` | datasheet |
| SFLP ODR 240 Hz | bits `100` | datasheet |
| SFLP ODR 480 Hz | bits `101` | datasheet |

---

## 8. 仍需实测确认的项目

这些不是未知资料缺口，而是硬件调参项：

- Active profile 选择 120 Hz 还是 240 Hz SFLP ODR。
- Suspend profile 是否关闭 SFLP，只保留 motion interrupt。
- Sleep profile wake threshold 与 wake duration。
- IMU INT 路由到 INT1 还是 INT2，以 PCB 为准。
- FIFO watermark 取值。
- BLE connection interval 是否维持 10 ms，或根据功耗调大。
- USB composite descriptor 最终组合方式。
- 2.4G dongle report packet 格式，需结合 CH592 dongle 固件设计。
