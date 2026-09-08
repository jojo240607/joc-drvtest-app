//! 用例注册表 + 顺序执行 + tick 预算 + 汇总报告 + 心跳任务。

use core::ffi::c_void;

use rtos_app_sdk::info;
use rtos_app_sdk::rtos::{msleep, spawn, tick_count, RTOS_PRIO_BLINK};

use crate::cases;
use crate::stacks;
use crate::verdict::{Ctx, Verdict};

/// 单个用例描述：分组 + 名字 + 预算（ms）+ 入口。
pub struct Case {
    pub group: &'static str,
    pub name: &'static str,
    pub budget_ms: u32,
    pub run: fn(&mut Ctx) -> Verdict,
}

/* ===========================================================================
 * 心跳任务：低优先级、每 500ms 一条。测试期间若整组卡死，验收脚本只见心跳
 * 消失而不见 REPORT——把「调度停滞」从「个别用例失败」中区分出来。
 * ========================================================================= */

extern "C" fn hb_entry(_arg: *mut c_void) {
    let mut n: u32 = 0;
    loop {
        n = n.wrapping_add(1);
        info!("drvtest", "hb n={} tick={}", n, tick_count());
        msleep(500);
    }
}

/* ===========================================================================
 * 运行器任务入口：拉心跳 → 顺序跑用例 → 输出汇总报告 → 保持存活。
 * ========================================================================= */

pub extern "C" fn main_entry(_arg: *mut c_void) {
    // 心跳任务（低优先级，证明测试期间调度活性）。
    unsafe {
        spawn(
            "drvtest_hb",
            hb_entry,
            RTOS_PRIO_BLINK,
            stacks::HB_STACK.as_mut_ptr(),
            stacks::HB_STACK.len(),
        );
    }

    let total = cases::CASES.len();
    let mut pass: u32 = 0;
    let mut fail: u32 = 0;
    let mut skip: u32 = 0;

    for c in cases::CASES.iter() {
        let mut ctx = Ctx::new(c.budget_ms);
        let v = (c.run)(&mut ctx);
        // 用例返回后校验是否超预算（warn 不阻断；卡死由心跳兜底暴露）。
        if ctx.over_budget() {
            info!(
                "drvtest",
                "[warn] {}.{} over budget ({}ms)", c.group, c.name, c.budget_ms
            );
        }
        match v {
            Verdict::Pass(d) => {
                pass += 1;
                info!("drvtest", "[pass] {}.{}: {}", c.group, c.name, d);
            }
            Verdict::Skip(d) => {
                skip += 1;
                info!("drvtest", "[skip] {}.{}: {}", c.group, c.name, d);
            }
            Verdict::Fail(d) => {
                fail += 1;
                info!("drvtest", "[fail] {}.{}: {}", c.group, c.name, d);
            }
        }
    }

    info!(
        "drvtest",
        "DRVTEST REPORT total={} pass={} fail={} skip={}", total, pass, fail, skip
    );

    // 报告后保持任务存活（验收脚本可观测稳定状态；心跳继续）。
    loop {
        msleep(1000);
    }
}
