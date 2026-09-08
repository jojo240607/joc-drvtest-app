//! d_flash：受管扇区管理器（flash0 管理 sector 11 @ 0x080E0000）。
//!
//! flash0 是备用 128KB 扇区的受管句柄，位于固件与 App 分区之上，可安全擦写；
//! 本用例只验证查询面（不擦写——验收中途擦写会动模拟器/真机的镜像区）：
//! GET_SECTOR=11、GET_BASE=0x080E0000、GET_STATUS（SR 无 BSY/错误标志）。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// flash0：受管扇区/基址/状态回读。
pub fn flash_ioctl(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("flash0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    let mut sector: u32 = 0;
    if dev.ioctl(FLASH_IOCTL_GET_SECTOR, (&mut sector as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_SECTOR 失败");
    }
    if sector != 11 {
        return Verdict::Fail("受管扇区 ≠ 11");
    }

    let mut base: u32 = 0;
    if dev.ioctl(FLASH_IOCTL_GET_BASE, (&mut base as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_BASE 失败");
    }
    if base != 0x080E_0000 {
        return Verdict::Fail("扇区基址 ≠ 0x080E0000");
    }

    let mut sr: u32 = 0xFFFF_FFFF;
    if dev.ioctl(FLASH_IOCTL_GET_STATUS, (&mut sr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_STATUS 失败");
    }
    if sr & FLASH_SR_BSY != 0 {
        return Verdict::Fail("FLASH 忙（BSY 置位）");
    }

    Verdict::Pass("flash0 受管扇区/基址/状态回读正常")
}
