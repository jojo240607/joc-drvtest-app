//! 用例结果模型 + 运行上下文（tick 预算）。
//!
//! 单核 RTOS 无法抢占一个卡死的用例，所以每个用例携带 tick 预算并**必须自限**：
//! 阻塞原语一律用 trywait / 超时 ioctl / 轮询 deadline；运行器在用例返回后校验
//! 是否超预算（记 warn，不停后续用例）。心跳任务负责把"整组卡死"暴露出来。

use rtos_app_sdk::rtos::tick_count;

/// 用例结果。`Skip` 必带 reason 字符串（能力缺失/平台限制）。
#[derive(Clone, Copy)]
pub enum Verdict {
    Pass(&'static str),
    Fail(&'static str),
    Skip(&'static str),
}

/// 用例运行上下文：记录进入时刻与预算，供用例自限与运行器超时判定。
pub struct Ctx {
    pub start_ticks: u32,
    pub budget_ticks: u32,
}

impl Ctx {
    /// 新建上下文；`budget_ms` 以 RTOS 1ms 节拍计。
    pub fn new(budget_ms: u32) -> Self {
        Ctx {
            start_ticks: tick_count(),
            budget_ticks: budget_ms,
        }
    }

    /// 剩余预算（ticks；<=0 表示已超时）。
    pub fn tick_left(&self) -> i32 {
        self.budget_ticks as i32 - tick_count().wrapping_sub(self.start_ticks) as i32
    }

    /// 是否已超出预算。
    pub fn over_budget(&self) -> bool {
        self.tick_left() <= 0
    }
}
