# 配置存储格式规范

设备配置在 DataFlash 中的持久化格式、双槽布局、校验规则和版本迁移。


## 1. 术语

### 1.1 Config Region

用于存放设备配置持久化数据的 DataFlash 区域。

### 1.2 Slot

Config Region 中的一个配置副本存储单元。

### 1.3 Active Slot

启动扫描后被判定为当前有效配置来源的 slot。

### 1.4 Payload

配置对象 `DeviceConfig` 经过编码后的字节序列。

### 1.5 Storage Version

外层 slot/header 协议版本。

### 1.6 Config Version

内层 `DeviceConfig` schema 版本。


## 2. 总体格式

配置存储采用双槽布局。

```text
Config Region
├── Slot A
│   ├── SlotHeader
│   └── Payload
└── Slot B
    ├── SlotHeader
    └── Payload
```

每个 slot 包含：

- 一个固定布局的 `SlotHeader`
- 一个变长 `Payload`
- 若干未使用保留空间


## 3. DataFlash 约束

本格式在 CH585 DataFlash / EEPROM 约束下定义。

### 3.1 已知平台参数

- DataFlash 总容量：32KB
- page size：256 bytes
- `EEPROM_MIN_ER_SIZE`：256 bytes
- `EEPROM_BLOCK_SIZE`：4096 bytes

### 3.2 对格式的约束

- Config Region 必须是平台可擦写区域。
- Slot 必须落在平台允许的 erase / write 边界内。
- Payload 不能超过 slot 可用容量。
- 写入 buffer 必须满足平台 API 的 RAM 对齐要求。

本规范不固定 Config Region 的绝对地址，由平台实现提供。


## 4. Slot 数量与布局

### 4.1 Slot 数量

固定为两个 slot：

- Slot A
- Slot B

### 4.2 Slot 容量

每个 slot 必须至少容纳：

- 一个完整 `SlotHeader`
- 一个完整 `Payload`

若编码后的 payload 超过 slot 可用容量，则该 payload 不得写入。

### 4.3 Slot 对齐

Slot 的起始地址与大小必须满足平台擦写约束。


## 5. SlotHeader

`SlotHeader` 是 slot 的固定前缀，用于描述 payload 的有效性、版本和选择顺序。

### 5.1 字段定义

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `magic` | `u32` | 固定 magic，标识这是 VoidPointer 配置 slot |
| `storage_version` | `u16` | 外层存储格式版本 |
| `config_version` | `u16` | payload 配置 schema 版本 |
| `payload_len` | `u32` | payload 长度，单位 byte |
| `sequence` | `u32` | 保存序号，越新越大 |
| `payload_crc32` | `u32` | payload 的 CRC32 |
| `header_crc32` | `u32` | header 的 CRC32，计算时该字段视为 0 |
| `flags` | `u32` | 预留标志位 |

### 5.2 magic

`magic` 固定为：

- ASCII 语义：`VCFG`
- little-endian 数值：`0x47464356`

### 5.3 storage_version

`storage_version` 描述以下内容的版本：

- `SlotHeader` 字段集合
- `SlotHeader` 编码规则
- slot 有效性判定规则
- 与 payload 的装配规则

### 5.4 config_version

`config_version` 描述 `DeviceConfig` schema 版本。

### 5.5 payload_len

- `payload_len` 是 payload 的实际长度。
- `payload_len` 必须大于 0。
- `payload_len` 必须不超过该 slot 的 payload 容量上限。

### 5.6 sequence

- `sequence` 用于在多个有效 slot 中选择更新的一份。
- `sequence` 必须在每次成功保存新配置后递增。
- `sequence` 的回绕比较规则当前规范不定义；达到上限后的处理由实现定义。

### 5.7 payload_crc32

- `payload_crc32` 覆盖整个 payload 字节序列。
- CRC 算法为标准 CRC32。

### 5.8 header_crc32

- `header_crc32` 覆盖整个 `SlotHeader`。
- 计算时 `header_crc32` 字段本身按 0 参与计算。

### 5.9 flags

- `flags` 为预留字段。
- 未定义标志位在当前版本中必须写 0。
- 当前版本读取时必须忽略未知标志位。


## 6. SlotHeader 编码

### 6.1 字节序

所有整数类型均使用 little-endian 编码。

### 6.2 布局

`SlotHeader` 使用固定字段顺序编码，顺序如下：

1. `magic`
2. `storage_version`
3. `config_version`
4. `payload_len`
5. `sequence`
6. `payload_crc32`
7. `header_crc32`
8. `flags`

### 6.3 Header 大小

`SlotHeader` 的编码大小是固定的，由上述字段唯一决定。


## 7. Payload 编码

### 7.1 编码方式

Payload 使用：

- `serde`
- `postcard`

对 `DeviceConfig` 进行编码。

### 7.2 编码边界

Payload 只承载 `DeviceConfig`。

Payload 不承载以下信息：

- slot 有效性
- payload 长度
- payload CRC
- sequence
- storage version

这些信息全部由外层 `SlotHeader` 承载。

### 7.3 有效 payload

只有同时满足以下条件时，payload 才能视为有效：

- `payload_len` 合法
- `payload_crc32` 校验通过
- `config_version` 可被当前固件处理
- `postcard` 反序列化成功
- 反序列化结果通过业务校验


## 8. Slot 有效性判定

扫描 slot 时，必须按以下顺序判定有效性：

1. `magic` 匹配
2. `storage_version` 被支持
3. `payload_len` 合法
4. `header_crc32` 校验通过
5. `payload_crc32` 校验通过
6. `config_version` 被支持或可迁移
7. `postcard` 反序列化成功
8. 配置业务校验成功

任一步失败，则该 slot 无效。

如果某个 slot 不能直接按当前 `config_version` 使用，但可通过受支持的 migration 路径转换为当前版本，则该 slot 在 Active Slot 选择时视为有效 slot。


## 9. Active Slot 选择规则

### 9.1 单槽有效

如果仅一个 slot 有效，则该 slot 为 Active Slot。

### 9.2 双槽有效

如果两个 slot 都有效，则选择 `sequence` 更大的 slot 作为 Active Slot。

如果两个有效 slot 的 `sequence` 相等，则固定选择 `Slot A` 作为 Active Slot。

### 9.3 双槽无效

如果两个 slot 都无效，则系统不得从 flash 应用配置，必须回退到默认配置。


## 10. 保存目标选择规则

保存时必须写入**当前 Active Slot 的另一槽**。

即：

- 当前 Active Slot 为 A，则下次保存写入 B
- 当前 Active Slot 为 B，则下次保存写入 A

保存成功后，新的目标槽成为新的 Active Slot。

如果当前不存在 Active Slot，且系统正在使用默认配置作为内存配置，则首次保存固定写入 `Slot A`。


## 11. Payload Schema

Payload 承载的是版本化 `DeviceConfig` 对象。

本规范只要求：

- payload 对应一个 `DeviceConfig`
- `DeviceConfig` 受 `config_version` 约束
- payload 的业务字段集合、默认值、取值范围和运行时语义由独立的配置 schema 文档定义

本规范不在此定义具体业务字段。


## 12. 默认配置

默认配置是当以下情况发生时使用的内存配置：

- 两个 slot 都无效
- `config_version` 不可处理
- payload 无法反序列化
- 业务校验失败

默认配置值由实现定义，但必须满足对应配置 schema 的全部约束。


## 13. 版本兼容性

### 13.1 storage_version

如果 `storage_version` 不被当前固件支持，则该 slot 无效。

### 13.2 config_version

如果 `config_version` 被当前固件支持，则直接按该 schema 解码。

如果 `config_version` 不被当前固件直接支持，但存在显式 migration 路径，则允许迁移后使用。

migration 必须基于受支持的旧版本 schema 执行。旧版本 schema 可通过保留有限旧版本结构体或等价的旧版本解码类型实现。

如果 `config_version` 既不被支持也不可迁移，则该 slot 无效。

### 13.3 migration 语义

- migration 采用显式版本迁移。
- migration 路径按相邻版本逐步执行，例如 `v3 -> v4 -> v5`。
- 不要求存在任意旧版本直接迁移到当前版本的捷径。
- migration 成功后，内存中的有效配置必须表现为当前 `config_version` 对应的 `DeviceConfig`。


## 14. 保存语义

### 14.1 写入单位

每次保存写入一个完整 slot 副本。

### 14.2 保存成功条件

一次保存只有在以下条件全部满足时才算成功：

- 目标 slot 擦除成功
- `SlotHeader` 写入成功
- payload 写入成功
- 回读校验成功

### 14.3 保存失败语义

如果目标 slot 写入失败，则原 Active Slot 仍保持有效，不得被此次失败写入破坏。

### 14.4 migration 后回写

如果配置通过 migration 生成当前版本的 `DeviceConfig`，则实现必须在后续允许写 flash 的上下文中，将该配置按当前 `config_version` 回写为新的 slot 副本。

当前版本直接解码成功时，不要求自动重写 payload。

如果 migration 已成功生成当前版本的 `DeviceConfig`，但回写失败，则本次运行仍继续使用该当前版本配置；回写失败只影响持久化状态，不得回退为旧版本运行时配置对象。


## 15. 错误分类

本格式允许实现至少区分以下错误类别：

- `StorageEmpty`
- `InvalidMagic`
- `UnsupportedStorageVersion`
- `InvalidPayloadLength`
- `HeaderCrcMismatch`
- `PayloadCrcMismatch`
- `UnsupportedConfigVersion`
- `DeserializeFailed`
- `ValidationFailed`
- `PayloadTooLarge`
- `FlashEraseFailed`
- `FlashWriteFailed`
- `ReadbackVerifyFailed`
- `MigrationFailed`

具体错误类型名可由实现定义，但语义不得弱于上述集合。


## 16. 读取语义

### 16.1 启动扫描

启动时必须至少扫描两个 slot 的 header，并根据本文档的有效性规则选择 Active Slot。

### 16.2 无有效 slot

若无有效 slot，则不得从 flash 应用配置，必须回退到默认配置。

### 16.3 迁移后的运行时结果

如果 slot 通过 migration 成功加载，则系统在运行时必须只暴露当前版本的 `DeviceConfig`，不得继续以旧版本结构作为运行时配置对象。


## 17. 未定义与保留行为

- 未定义的 `flags` 位在当前版本中保留。
- 未定义的 enum 值必须由对应 schema 文档定义其处理方式；若未定义，则视为无效配置值。
- 超出 schema 支持范围的数值必须视为校验失败或经显式 sanitize 后再应用。


## 18. 规范结论

VoidPointer 配置持久化格式的规范结论如下：

- 使用双槽存储配置副本
- 使用固定布局 `SlotHeader`
- 使用 little-endian 整数编码
- 使用 `CRC32` 校验 header 与 payload
- 使用 `serde + postcard` 编码 `DeviceConfig`
- 使用 `sequence` 在多个有效 slot 中选择更新副本
- 使用默认配置处理双槽都无效的情况
- 使用 `storage_version` 与 `config_version` 管理兼容性
