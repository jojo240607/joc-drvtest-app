//! d_pwm：PWM 通道（pwm0 = TIM3_CH1 @400Hz/84MHz → 周期 210000 ticks）。
//! 判据：周期回读 >0；SET_DUTY_PERCENT(50%) 后 duty 回读 ≈ 周期一半（±5%）。

use core::ptr::null_mut;

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// pwm0：周期/占空比 ioctl 读写一致 + 通道使能/禁用。
pub fn pwm_ioctl(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("pwm0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 周期回读（400Hz @ 84MHz → ~210000 ticks）。
    let mut period: u32 = 0;
    if dev.ioctl(PWM_IOCTL_GET_PERIOD_TICKS, (&mut period as *mut u32).cast()) != 0 || period == 0 {
        return Verdict::Fail("GET_PERIOD_TICKS 失败/为 0");
    }

    // 50% 占空比 → duty 回读 ≈ period/2（±5%）。
    let mut duty: i32 = 50;
    if dev.ioctl(PWM_IOCTL_SET_DUTY_PERCENT, (&mut duty as *mut i32).cast()) != 0 {
        return Verdict::Fail("SET_DUTY_PERCENT 失败");
    }
    let mut dticks: u32 = 0;
    if dev.ioctl(PWM_IOCTL_GET_DUTY_TICKS, (&mut dticks as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_DUTY_TICKS 失败");
    }
    let half = period / 2;
    let tol = period / 20; // 5%
    if dticks.abs_diff(half) > tol {
        return Verdict::Fail("50% 占空比回读偏差过大");
    }

    // 通道使能/禁用。
    if dev.ioctl(PWM_IOCTL_ENABLE_CHANNEL, null_mut()) != 0 {
        return Verdict::Fail("ENABLE_CHANNEL 失败");
    }
    if dev.ioctl(PWM_IOCTL_DISABLE_CHANNEL, null_mut()) != 0 {
        return Verdict::Fail("DISABLE_CHANNEL 失败");
    }

    Verdict::Pass("pwm0 周期/占空比 ioctl 读写一致 + 通道开关正常")
}
