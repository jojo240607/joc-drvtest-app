//! d_crc：硬件 CRC32 引擎（STM32 固定多项式 0x04C11DB7、初值 0xFFFFFFFF、
//! 不反射、无最终异或 = CRC-32/MPEG-2 变体）。判据为软件模型逐位对齐的
//! **数值正确**（见下方 `crc32_mpeg2_word`；已用 0x0376E6E7 校验值验证模型）。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// 软件 CRC-32/MPEG-2 单字推进（MSB 先行 = STM32 CRC 硬件语义）。
/// 校验：crc32_mpeg2_word 链式喂 "123456789" 的字节 == 0x0376E6E7。
fn crc32_mpeg2_word(mut crc: u32, word: u32) -> u32 {
    // 按大端字节序喂入（硬件 DR 写全字时 32 位 MSB 先入）。
    let bytes = word.to_be_bytes();
    for b in bytes {
        crc ^= (b as u32) << 24;
        for _ in 0..8 {
            if crc & 0x8000_0000 != 0 {
                crc = (crc << 1) ^ 0x04C1_1DB7;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// crc0：RESET → RESULT=0xFFFFFFFF → 逐字 UPDATE → 与软件模型比对。
pub fn crc32_known(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("crc0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // RESET 后 DR 应读回初值 0xFFFFFFFF。
    if dev.ioctl(CRC_IOCTL_RESET, core::ptr::null_mut()) != 0 {
        return Verdict::Fail("CRC_RESET 失败");
    }
    let mut r0: u32 = 0;
    if dev.ioctl(CRC_IOCTL_RESULT, (&mut r0 as *mut u32).cast()) != 0 || r0 != 0xFFFF_FFFF {
        return Verdict::Fail("RESET 后初值 ≠ 0xFFFFFFFF");
    }

    // 喂 0x12345678 → 期望 0xDF8A8A2B。
    let mut w1: u32 = 0x1234_5678;
    if dev.ioctl(CRC_IOCTL_UPDATE, (&mut w1 as *mut u32).cast()) != 0 {
        return Verdict::Fail("CRC_UPDATE 失败");
    }
    let mut r1: u32 = 0;
    if dev.ioctl(CRC_IOCTL_RESULT, (&mut r1 as *mut u32).cast()) != 0 {
        return Verdict::Fail("CRC_RESULT#1 失败");
    }
    let expect1 = crc32_mpeg2_word(0xFFFF_FFFF, w1);
    if r1 != expect1 {
        return Verdict::Fail("CRC(0x12345678) 与软件模型不一致");
    }

    // 再喂 0x87654321（链式）→ 期望 0xC15A147D。
    let mut w2: u32 = 0x8765_4321;
    if dev.ioctl(CRC_IOCTL_UPDATE, (&mut w2 as *mut u32).cast()) != 0 {
        return Verdict::Fail("CRC_UPDATE#2 失败");
    }
    let mut r2: u32 = 0;
    if dev.ioctl(CRC_IOCTL_RESULT, (&mut r2 as *mut u32).cast()) != 0 {
        return Verdict::Fail("CRC_RESULT#2 失败");
    }
    let expect2 = crc32_mpeg2_word(expect1, w2);
    if r2 != expect2 {
        return Verdict::Fail("链式 CRC 与软件模型不一致");
    }

    Verdict::Pass("crc0 硬件 CRC32 与软件模型逐值一致")
}
