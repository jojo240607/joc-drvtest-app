# joc-drvtest-app — jOS RTOS 驱动验证工程

独立仓库的驱动验证应用：以 **joc-rtos-app-sdk** 为 SDK 编译成 `app.bin`，
由 jOS 系统分区（`joc-base`）经 `rust_app_start` 挂载，跑在 RTOS 应用分区上，
逐组验证板载驱动"能否跑通"，最后输出一份可被模拟器验收抓取的报告。

```
┌─────────────────────────── jOS 双分区 ───────────────────────────┐
│  系统分区 (joc-base build_rel/stm32f407_minimal.elf)              │
│    g_app_slot 函数指针表（app 不直接链接裸 RTOS 符号/不碰裸寄存器） │
│  应用分区 (本工程 app.bin)                                        │
│    runner 任务 → 依次跑 21 个用例 → DRVTEST REPORT                 │
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

## 用例清单（21 个，15 组驱动）

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

## v2 延后清单

usb（open 慢/模型缺）、sdio/sd_card、flash、can、iwdg/wwdg、i2s、fsmc、dcmi，
以及 uart DMA TX 写路径在真机上的完整判据。
