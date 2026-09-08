//! drvtest-app：jOS 驱动验证应用（轨 B 应用分区）。
//!
//! 以 `joc-rtos-app-sdk` 为 SDK，经 g_app_slot 服务表间接调用内核与设备，
//! **绝不直接链接裸 RTOS 符号 / 不碰裸寄存器**。核心目标：逐个驱动「跑通」验证——
//! open 成功、read/write/ioctl 得到预期结果、中断路径能推进——并输出统一
//! pass/fail/skip 报告（`DRVTEST REPORT total=.. pass=.. fail=.. skip=..`），
//! mcu_simulater 验收脚本据此判定（fail=0）。
//!
//! 结构：
//!   - `runner`：用例注册表 + 顺序执行 + tick 预算 + 汇总报告 + 心跳任务
//!   - `cases/`：按驱动分组的用例（k_sdk / d_uart / d_gpio / d_adc / ...）
//!   - `cap`：能力探测（设备缺失/open 失败/模拟器已知阻塞器 → Skip 决策）
//!   - `ioctl`：驱动私有 ioctl 命令常量（镜像 joc-base tools/abi/rtos_abi_ioctl.h）
//!   - `verdict`：Verdict{Pass/Fail/Skip} + Ctx（tick 预算）
//!
//! 约束：no_std、无堆、静态栈（.rust_bss）；用例必须自限（阻塞原语用 trywait/
//! 超时 ioctl），单核无法抢占卡死用例——心跳任务兜底暴露停滞。

#![no_std]
#![allow(static_mut_refs)]

mod cap;
mod cases;
mod runner;
mod stacks;
mod verdict;

use rtos_app_sdk::info;
use rtos_app_sdk::rtos::{spawn, RTOS_PRIO_BH_MED};

/// 应用入口（SDK 的 rust_app_start 调用）：拉起驱动验证运行器任务后返回 0。
/// 运行器任务顺序执行所有用例并输出报告；报告后自身保持存活（心跳继续）。
#[no_mangle]
pub extern "C" fn app_main() -> i32 {
    unsafe {
        spawn(
            "drvtest",
            runner::main_entry,
            RTOS_PRIO_BH_MED,
            stacks::RUNNER_STACK.as_mut_ptr(),
            stacks::RUNNER_STACK.len(),
        );
    }
    info!("drvtest", "runner spawned (link verified)");
    0
}
