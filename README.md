# joc-drvtest-app — jOS RTOS 驱动验证工程

独立仓库的驱动验证应用：以 **joc-rtos-app-sdk** 为 SDK 编译成 `app.bin`，
由 jOS 系统分区（`joc-base`）经 `rust_app_start` 挂载，跑在 RTOS 应用分区上，
逐组验证板载驱动"能否跑通"，最后输出一份可被模拟器验收抓取的报告。

```
┌─────────────────────────── jOS 双分区 ───────────────────────────┐
│  系统分区 (joc-base build_rel/stm32f407_minimal.elf)              │
│    g_app_slot 函数指针表（app 不直接链接裸 RTOS 符号/不碰裸寄存器） │
│  应用分区 (本工程 app.bin)                                        │
│    runner 任务 → 依次跑 28 个用例 → DRVTEST REPORT                 │
└───────────────────────────────────────────────────────────────────┘
```

## 目录结构

```
apps/drvtest/
  src/lib.rs          # no_std 入口：app_main spawn runner 任务
  src/cap.rs          # 设备探测：get/open，缺失或 open 失败 → Skip
  src/verdict.rs      # Pass/Fail/Skip 三态 + 预算（Ctx::new(budget_ms)）
  src/runner.rs       # 用例框架：遍历执行、超预算告警、报告输出、心跳兜底
  src/stacks.rs       # .rust_bss 静态栈（runner 8K / 心跳 2K / worker 2K）
  src/cases/          # 用例分组注册表（cases/mod.rs）+ 各组驱动用例
  src/cases/k_sdk.rs  # SDK/RTOS 地基：tick/msleep、spawn+信号量、互斥量、设备表
  src/cases/d_*.rs    # 驱动用例：uart/gpio/adc/temp/rng/crc/rtc/timer/exti/pwm/dac/spi_i2c/dma
linker/app.ld         # APP_FLASH 0x08060000 / APP_RAM 0x20004000（与 SDK 一致）
build_app.py          # cargo build -p drvtest-app → gcc 链接 → objcopy app.bin
.cargo/config.toml    # thumbv7em-none-eabihf / cortex-m4 / relocation-model=static
```

## 构建

```bash
cd joc-drvtest-app
# 依赖：同机 joc-rtos-app-sdk 工作区（Cargo.toml path 依赖 ../../../joc-rtos-app-sdk/sdk）
python3 build_app.py --app apps/drvtest
# 产出 app.bin（约 13KB，远小于 384KB 分区）
```

## 运行与验收

**模拟器一键跑**（mcu_simulater）：

```bash
cd mcu_simulater
cargo run --release --bin cfg-run -- --cfg <(cat <<'EOF'
elf = /home/ubuntu/work/joc-base/build_rel/stm32f407_minimal.elf
app = /home/ubuntu/work/joc-drvtest-app/app.bin
n = 400000
max_steps = 0
EOF
)
```

**自动化验收**：`cargo test --release --test x_drvtest`（mcu_simulater 仓库内）。
断言：App 挂载 + 心跳出现 + `DRVTEST REPORT fail=0`，INSN_INVALID 兜底判失败。

## 用例清单（28 个，21 组驱动）

### v1（21 个，15 组）

| 组 | 用例 | 判据 |
|---|---|---|
| k_sdk | tick_msleep / spawn_sem / mutex_roundtrip / slot_devtable | RTOS 地基：时间推进、任务/信号量/互斥量、设备服务表 |
| d_uart | uart0_console / uart_others | 控制台写+波特率 ioctl 往返+非阻塞读；uart1/2/3 open+ioctl |
| d_gpio | output_roundtrip / input_probe | 输出脚写读往返+toggle；输入脚 open+读 |
| d_adc | adc0_read | POLL 引擎单次转换，12 位值域合法 |
| d_temp | temp0_read | 温度 float 在 -40..125°C |
| d_rng | rng_read | 两次读数不同且非全 0 |
| d_crc | crc32_known | 硬件 CRC 与软件 CRC-32/MPEG-2 模型**逐值一致** |
| d_rtc | rtc_time | 时间字段合法 + SET/GET 往返（秒偏移≤2）+ 日期合法 |
| d_timer | timer_overflow / timer_app_isr | 溢出计数推进；App ISR 经 irq_attach 收到 TIM2 溢出 |
| d_exti | exti_trigger | 软件触发→驱动+App 共享线 ISR 均收到 |
| d_pwm | pwm_ioctl | 周期回读 + 50% 占空比回读≈周期一半（±5%）+ 通道开关 |
| d_dac | dac_ioctl | SET/GET_VALUE 往返一致 |
| d_spi_i2c | i2c_scan / spi_cr1 | I2C 扫描完成+CCR 回读；SPI 任一总线 open+CR1 回读 |
| d_dma | dma_pool | 流池管理器 open 成功（深层使用由 timer0 预留流间接覆盖） |

### v2（7 个，6 组，延后清单实施）

| 组 | 用例 | 判据 |
|---|---|---|
| d_can | can_loopback | can0（BTR.LBKM 回环）SEND_FRAME 0x123 8 字节 → RECV 逐字段比对一致；TSR.TXOK0 置位；非回环模式则 Skip |
| d_flash | flash_ioctl | 受管扇区回读 == 11、基址 == 0x080E0000、状态寄存器无 BSY（不擦写） |
| d_wdg | iwdg_config / wwdg_config | 预分频/重装载/窗口 ioctl SET→GET 往返一致；**绝不 START**（一旦武装无法停止，超时复位系统会丢验收） |
| d_i2s | i2s_config | I2SCFGR.I2SE 使能 + 音频时钟 48kHz 回读 + 分频非 0 + PLLI2S 就绪 |
| d_sd_card | sdio_sd_init | sdio0 open 后 CLKCR（CLKDIV=118/CLKEN/PWRCTRL/WIDBUS 4-bit）回读；sd_card0 SD_CARD_IOCTL_INIT 初始化序列跑通（虚拟卡就绪） |
| d_usb | usb_ioctl | usb0 open + 核心寄存器回读 + 固件 USB 栈自测（RUN_CTRL_SELFTEST，纯软件合成控制传输，不依赖物理主机）== 0 |

## 判据设计原则

- **数值正确**：能用软件模型/回读对得上的就逐值比对（d_crc 对软件模型、
  d_pwm 占空比回读、d_dac 往返、d_rtc 回读、d_gpio 写读往返）。
- **跑通**：无对拍基准的，验证链路可达 + 值域合法（d_adc 12 位值域、d_temp 温度范围、
  d_rng 随机性、d_timer 计数推进）。
- **Skip 三态**：设备缺失 / open 失败（平台引脚占用，如 spi0 的 PA6 被 pwm0 占用）/ 
  平台无能力（如 irq_attach 不可用）→ 跳过并注明原因，不算失败。
- **模拟器已知阻塞器在用例内绕开**：uart1/2/3 不写（DMA TX 阻塞）、adc 先切 POLL、
  spi/i2c 不做需从机应答的传输。

## 框架约定（自限性）

- 每个用例有独立预算（`Ctx::new(budget_ms)`），超预算打 `[warn] ... over budget`；
- 用例必须自限：阻塞等待一律用 trywait 轮询 + 死线 / 预算内完成，绝不允许
  无限期挂起（单核无法抢占卡死用例）；
- 心跳任务（500ms 一拍）兜底暴露整组卡死；
- 报告行 `DRVTEST [pass|fail|skip] 组.名: detail` + 结尾
  `DRVTEST REPORT total=.. pass=.. fail=.. skip=..`（验收 grep fail=0）。

## 新增驱动用例

1. `apps/drvtest/src/cases/d_xxx.rs` 写用例函数 `fn xxx(_ctx: &mut Ctx) -> Verdict`；
2. `cases/mod.rs` 加 `pub mod d_xxx;` + 注册 `Case { group, name, budget_ms, run }`；
3. 判据尽量"数值正确"；拿不准的链路先按跑通写，模拟器跑一遍再收紧。

## 依赖与协作

- **joc-rtos-app-sdk**：ioctl 常量、irq 注册、sem_trywait 等由 SDK 提供单点真源；
- **joc-base**：系统分区固件（驱动 + g_app_slot）；驱动/IRQ 语义见其 `drv/*` 与 `app_slot/`；
- **mcu_simulater**：`tests/x_drvtest.rs` 是 CI 验收；模拟器缺口（如 RTC 的 LSI、
  出厂校准字、DAC 无触发转 DOR）会随工程暴露并修复。

## v2 实施记录（can/flash/iwdg/wwdg/i2s/sdio/sd_card/usb）

v1 批准的延后清单已实施完毕：can、flash、iwdg/wwdg、i2s、sdio/sd_card、usb 全部跑通
（`DRVTEST REPORT total=28 pass=28 fail=0 skip=0`）。原清单中的 **dcmi/fsmc 未注册
board 节点**，本期不实施（无设备可验证）。

v2 暴露并修复的模拟器缺口（对齐 F407 真机语义，而非绕过判据）：

| 缺口 | 说明 |
|---|---|
| CAN 回环路由死锁 | machine 订阅回调对发送端二次加锁导致重入死锁；改为在 `Can::transmit` 锁内直接 `feed_rx` 完成回环 |
| CAN DLC 位布局 | 发送邮箱 TDTR / 接收邮箱 RDT0R 的 `DLC[3:0]` 在 **bits 0:3**（bxCAN 权威，svd2rust 佐证），非 bit19:16 |
| CAN_BTR_LBKM | F407 回环模式位是 **bit30**（SILM=bit31），SDK 常量与模拟器据此对齐 |
| SDIO 状态寄存器偏移 | F4 布局 **STA@0x34 / ICR@0x38 / MASK@0x3C / FIFOCNT@0x48**（非 0x38/0x3C/0x40/0x44）；偏移错位曾使固件 `r->STA` 落空→命令恒超时 |
| SDIO ACMD41 OCR | 虚拟卡就绪需置 **bit31（上电完成/busy 结束）**+bit30（CCS），否则 sc_init 轮询超时 |
| SDIO_CLKCR 位 | F4 布局 CLKDIV[7:0]、**CLKEN=bit8**、WIDBUS[12:11]（用例回读据此断言） |
| FLASH 寄存器区 | 新增 FLASH 外设模型（KEYR/SR/CR），flash0 的 GET_SECTOR/GET_BASE/GET_STATUS 可回读 |
| RCC PLLI2S | 新增 PLLI2SON→PLLI2SRDY 立即就绪，i2s0 的 48kHz/PLL 链路可回读 |

固件（joc-base）在 v2 中**未改动**——以上均为模拟器建模错误或 SDK 常量错误，已被
验证工程暴露并修正（这正是"驱动验证"的价值：验证方与被测方交叉对拍，纠出建模偏差）。
