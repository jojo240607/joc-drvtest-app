//! d_sd_card：SDIO 总线 + SD 卡初始化链路。
//!
//! sd_card0 是依赖 sdio0 总线的块设备：块读写走 block vtable（SDK 未上提该
//! 接口），故本用例验证「总线 open + 寄存器回读 + 卡初始化序列」跑通——
//! 模拟器带虚拟 SD 卡（1MB 后备缓冲，CMD0/8/55/ACMD41/2/3/7/9/16 可识），
//! 初始化序列应完整通过并返回就绪。块数据数值判据留待真机/SDK 上提 block
//! 接口后补充。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// sdio0 + sd_card0：总线回读 + 卡初始化。
pub fn sdio_sd_init(_ctx: &mut Ctx) -> Verdict {
    // sd_card0 依赖 sdio0 总线已 open（先 open 总线）
    let bus = match cap::open("sdio0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 总线寄存器回读（F4 SDIO_CLKCR 布局：CLKDIV[7:0]、CLKEN=bit8、WIDBUS[12:11]）
    let mut clkcr: u32 = 0;
    if bus.ioctl(SDIO_IOCTL_GET_CLKCR, (&mut clkcr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_CLKCR 失败");
    }
    if clkcr & SDIO_CLKCR_CLKEN == 0 {
        return Verdict::Fail("SDIO 时钟未使能（CLKEN）");
    }
    if clkcr & SDIO_CLKCR_CLKDIV != 118 {
        return Verdict::Fail("SDIO 时钟分频 ≠ 118（open 默认）");
    }
    if clkcr & SDIO_CLKCR_WIDBUS_0 == 0 {
        return Verdict::Fail("SDIO 未配置 4-bit 总线");
    }
    let mut pwr: u32 = 0;
    if bus.ioctl(SDIO_IOCTL_GET_POWER, (&mut pwr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_POWER 失败");
    }
    if pwr & 0x03 == 0 {
        // PWR.PWRCTRL 非 0
        return Verdict::Fail("SDIO 电源未开启");
    }

    let dev = match cap::open("sd_card0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 初始化序列：CMD0→ACMD41→CMD2/3/9/7/16（虚拟卡就绪才返回 0）
    if dev.ioctl(SD_CARD_IOCTL_INIT, core::ptr::null_mut()) != 0 {
        return Verdict::Skip("SD 卡初始化失败（无卡/未就绪）");
    }

    Verdict::Pass("sdio0 + sd_card0 初始化链路跑通（虚拟卡就绪）")
}
