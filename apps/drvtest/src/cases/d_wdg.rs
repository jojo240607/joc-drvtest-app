//! d_wdg：看门狗配置面（IWDG 独立 + WWDG 窗口）。
//!
//! 关键设计约束：IWDG/WWDG 一旦 START（ARM）即无法停止，超时未喂会复位整个
//! 系统——验收中途复位会丢失全部用例结果（也打乱心跳）。故本组用例只验证
//! 配置 ioctl 往返（SET/GET）与状态回读，**不 START**；真机上由应用自行决定
//! 何时武装看门狗并喂狗。复位路径的数值行为已在模拟器 wdog 单测覆盖。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// iwdg0：预分频/重装载 SET→GET 往返 + 状态回读（未启动）。
pub fn iwdg_config(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("iwdg0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    let mut pr: u32 = 3; // 分频 4<<3 = 32
    if dev.ioctl(IWDG_IOCTL_SET_PRESCALER, (&mut pr as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_PRESCALER 失败");
    }
    let mut rl: u32 = 2047;
    if dev.ioctl(IWDG_IOCTL_SET_RELOAD, (&mut rl as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_RELOAD 失败");
    }

    let mut gpr: u32 = 0;
    if dev.ioctl(IWDG_IOCTL_GET_PRESCALER, (&mut gpr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_PRESCALER 失败");
    }
    if gpr != pr {
        return Verdict::Fail("预分频未回读一致");
    }
    let mut grl: u32 = 0;
    if dev.ioctl(IWDG_IOCTL_GET_RELOAD, (&mut grl as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_RELOAD 失败");
    }
    if grl != rl {
        return Verdict::Fail("重装载未回读一致");
    }

    let mut sr: u32 = 0;
    if dev.ioctl(IWDG_IOCTL_GET_STATUS, (&mut sr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_STATUS 失败");
    }

    Verdict::Pass("iwdg0 预分频/重装载 ioctl 往返一致（未启动，避免复位）")
}

/// wwdg0：预分频/窗口 SET→GET 往返 + 计数器/状态回读（未启动）。
pub fn wwdg_config(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("wwdg0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    let mut tb: u32 = 1; // WDGTB=1（分频 4096×2）
    if dev.ioctl(WWDG_IOCTL_SET_PRESCALER, (&mut tb as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_PRESCALER 失败");
    }
    let mut win: u32 = 0x60;
    if dev.ioctl(WWDG_IOCTL_SET_WINDOW, (&mut win as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_WINDOW 失败");
    }

    let mut cfg: u32 = 0;
    if dev.ioctl(WWDG_IOCTL_GET_CONFIG, (&mut cfg as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_CONFIG 失败");
    }
    // CFR：WDGTB=[8:7]（1<<7=128），W=[6:0]（0x60）
    if cfg & (1 << 7) == 0 || (cfg & 0x7F) != 0x60 {
        return Verdict::Fail("窗口配置未回读一致");
    }

    let mut cnt: u32 = 0;
    if dev.ioctl(WWDG_IOCTL_GET_COUNTER, (&mut cnt as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_COUNTER 失败");
    }
    let mut sr: u32 = 0;
    if dev.ioctl(WWDG_IOCTL_GET_STATUS, (&mut sr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_STATUS 失败");
    }

    Verdict::Pass("wwdg0 预分频/窗口 ioctl 往返一致（未启动，避免复位）")
}
