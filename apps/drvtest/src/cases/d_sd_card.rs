//! d_sd_card：SDIO 总线 + SD 卡初始化链路 + 块读写。
//!
//! sd_card0 是依赖 sdio0 总线的块设备。初始化序列（CMD0/8/55/ACMD41/2/3/7/9/16）
//! 跑通后经 SD_CARD_IOCTL_READ/WRITE_BLOCK（C 类上提的 ioctl 块面）做扇区
//! 写读往返——模拟器带虚拟 SD 卡（1MB 后备缓冲，CMD17/24 数据路径经
//! DCTRL+DMA 搬运），判据数值一致。

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

/// sd_card0：扇区 0 写读往返（512B，经 SDIO CMD24/CMD17 数据路径）。
pub fn block_rw(_ctx: &mut Ctx) -> Verdict {
    // 数据路径走 DMA 引擎：sdio0 open 默认 POLL（dma_s 未分配），先切 DMA
    //（与 d_adc 切 POLL 同理——SDIO CMD_DATA 的 DCTRL+DMA 搬运需 DMA 引擎）。
    let bus = match cap::open("sdio0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut mode: u32 = STREAM_MODE_DMA;
    if bus.ioctl(STREAM_IOCTL_SET_MODE, (&mut mode as *mut u32).cast()) != 0 {
        return Verdict::Skip("sdio0 切 DMA 引擎失败");
    }

    let dev = match cap::open("sd_card0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 确保卡就绪（重复 INIT 可跑：虚拟卡支持重初始化）
    if dev.ioctl(SD_CARD_IOCTL_INIT, core::ptr::null_mut()) != 0 {
        return Verdict::Skip("SD 卡初始化失败（无卡/未就绪）");
    }

    // 写扇区 0：512 字节非平凡 pattern
    let mut pat = [0u8; 512];
    for i in 0..512 {
        pat[i] = (i as u8).wrapping_mul(7).wrapping_add(3);
    }
    let mut bio = SdBlockIo { lba: 0, count: 1, buf: pat.as_mut_ptr() };
    let wrc = dev.ioctl(SD_CARD_IOCTL_WRITE_BLOCK, (&mut bio as *mut SdBlockIo).cast());
    if wrc != 0 {
        return Verdict::Fail("WRITE_BLOCK 失败");
    }

    let mut rd = [0u8; 512];
    let mut bio2 = SdBlockIo { lba: 0, count: 1, buf: rd.as_mut_ptr() };
    let rrc = dev.ioctl(SD_CARD_IOCTL_READ_BLOCK, (&mut bio2 as *mut SdBlockIo).cast());
    if rrc != 0 {
        return Verdict::Fail("READ_BLOCK 失败");
    }

    if rd != pat {
        return Verdict::Fail("扇区 0 写读往返不一致");
    }
    Verdict::Pass("sd_card0 扇区 0 写读往返一致（512B，CMD24/CMD17 数据路径）")
}
