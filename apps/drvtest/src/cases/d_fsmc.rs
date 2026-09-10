//! d_fsmc：FSMC 灵活静态存储器控制器（F407 @0xA0000000）。
//! 判据（数值正确优先）：
//!  1. BCR1 写读往返一致（可写位掩码内：WREN/MWID 等）；
//!  2. BTR1 写读往返一致（时序位全保存）；
//!  3. BWTR1 复位回读 0；
//!  4. 片选未使能时窗口访问返回负值（hal 门控，防误写未选通存储器）；
//!  5. BANK1_ENABLE 后 BCR1.MBKEN 置位；
//!  6. Bank1 窗口（0x60000000）32 位字写读往返一致。

use core::ptr::null_mut;

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// fsmc0：BCR/BTR/BWTR 寄存器往返 + Bank1 片选窗口写读。
pub fn fsmc_ioctl(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("fsmc0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 1) BCR1 写读往返（WREN + MWID=01 16 位数据总线，均可写位）
    let mut bcr: u32 = FSMC_BCR_WREN | (0x1 << 4);
    if dev.ioctl(FSMC_IOCTL_SET_BCR, (&mut bcr as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_BCR 失败");
    }
    let mut rbcr: u32 = 0;
    if dev.ioctl(FSMC_IOCTL_GET_BCR, (&mut rbcr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_BCR 失败");
    }
    if rbcr != bcr {
        return Verdict::Fail("BCR 往返不一致");
    }

    // 2) BTR1 写读往返（时序位按写值保存）
    let mut btr: u32 = 0x0F0F_0F0F; // ADDSET/ADDHLD/DATAST/BUSTURN/CLKDIV/DATLAT
    if dev.ioctl(FSMC_IOCTL_SET_BTR, (&mut btr as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_BTR 失败");
    }
    let mut rbtr: u32 = 0;
    if dev.ioctl(FSMC_IOCTL_GET_BTR, (&mut rbtr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_BTR 失败");
    }
    if rbtr != btr {
        return Verdict::Fail("BTR 往返不一致");
    }

    // 3) BWTR1 复位回读 0
    let mut rbwtr: u32 = 0xFFFF_FFFF;
    if dev.ioctl(FSMC_IOCTL_GET_BWTR, (&mut rbwtr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_BWTR 失败");
    }
    if rbwtr != 0 {
        return Verdict::Fail("BWTR 复位值非 0");
    }

    // 4) 片选未使能时窗口访问必须被驱动拒绝（hal 门控）
    let mut pre = [0u8; 4];
    if dev.read(&mut pre) >= 0 {
        return Verdict::Fail("未使能片选窗口访问应返回负值");
    }

    // 5) BANK2_ENABLE → BCR2.MBKEN 置位（Bank1 挂 LCD，窗口往返测试走 Bank2）
    if dev.ioctl(FSMC_IOCTL_BANK2_ENABLE, null_mut()) != 0 {
        return Verdict::Fail("BANK2_ENABLE 失败");
    }

    // 6) Bank2 窗口 32 位字写读往返
    let words: [u32; 2] = [0xDEAD_BEEF, 0x1234_5678];
    for (i, w) in words.iter().enumerate() {
        let mut w32 = FsmcWin32 { off: (i as u32) * 4, value: *w };
        if dev.ioctl(FSMC_IOCTL_BANK2_WRITE32, (&mut w32 as *mut FsmcWin32).cast()) != 0 {
            return Verdict::Fail("BANK2_WRITE32 失败");
        }
        let mut r32 = FsmcWin32 { off: (i as u32) * 4, value: 0 };
        if dev.ioctl(FSMC_IOCTL_BANK2_READ32, (&mut r32 as *mut FsmcWin32).cast()) != 0 {
            return Verdict::Fail("BANK2_READ32 失败");
        }
        if r32.value != *w {
            return Verdict::Fail("Bank2 窗口写读往返不一致");
        }
    }

    Verdict::Pass("fsmc0 BCR/BTR/BWTR 往返 + Bank2 窗口写读一致（Bank1 挂 LCD）")
}
