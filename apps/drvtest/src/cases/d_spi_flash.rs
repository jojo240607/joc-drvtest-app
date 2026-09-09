//! d_spi_flash：SPI NOR Flash（spi_flash0，W25Q128 类）保存/存储全链路。
//! 判据（与模拟器虚拟从机 flash.rs 同一协议）：
//!   JEDEC ID：0xEF 40 18（0x9F 命令回 3B）
//!   写读回：页编程（WREN + 0x02 + 3B 地址）后 READ（0x03）读回一致
//!   扇区擦除：写 → 擦 4KB → 读回全 0xFF
//!   持久化标记：固定地址写特征串（x_drvtest 宿主机据此校验文件映像存活）

use rtos_app_sdk::device::Device;
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{FlashIo, FLASH_IOCTL_ERASE_SECTOR, FLASH_IOCTL_GET_JEDEC, FLASH_IOCTL_READ, FLASH_IOCTL_WRITE};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// W25Q128 JEDEC ID（大端 u24）
const JEDEC_W25Q128: u32 = 0xEF4018;
/// 页编程特征串（含持久化标记）
const MARKER: &[u8] = b"SPIFLASH-SAVE";
const MARKER_ADDR: u32 = 0x0300;

fn open_flash() -> Result<Device, &'static str> {
    cap::open("spi_flash0")
}

fn rd(dev: &Device, addr: u32, buf: &mut [u8]) -> i32 {
    let io = FlashIo { addr, len: buf.len() as u16, buf: buf.as_mut_ptr() };
    dev.ioctl(FLASH_IOCTL_READ, (&io as *const FlashIo as *mut FlashIo).cast())
}

fn wr(dev: &Device, addr: u32, buf: &[u8]) -> i32 {
    let io = FlashIo { addr, len: buf.len() as u16, buf: buf.as_ptr() as *mut u8 };
    dev.ioctl(FLASH_IOCTL_WRITE, (&io as *const FlashIo as *mut FlashIo).cast())
}

pub fn jedec_id(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_flash() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut id: u32 = 0;
    if dev.ioctl(FLASH_IOCTL_GET_JEDEC, (&mut id as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_JEDEC 失败");
    }
    if id != JEDEC_W25Q128 {
        return Verdict::Fail("JEDEC ID 不符");
    }
    info!("spi_flash", "JEDEC=0x{id:06X} 正确");
    Verdict::Pass("JEDEC=EF4018")
}

pub fn write_read_back(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_flash() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    // 0x2000（扇区 2）——避开 erase_sector 用例擦除的扇区 0（0x0000-0x0FFF），
    // 保证持久化验收时该数据仍存活（跨扇区保存证明）。
    const ADDR: u32 = 0x2000;
    let pat: [u8; 8] = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67];
    if wr(&dev, ADDR, &pat) != 0 {
        return Verdict::Fail("页编程失败");
    }
    let mut back = [0u8; 8];
    if rd(&dev, ADDR, &mut back) != 0 {
        return Verdict::Fail("READ 失败");
    }
    if back != pat {
        return Verdict::Fail("写读回不一致");
    }
    info!("spi_flash", "写读回 8B 一致");
    Verdict::Pass("写读回一致")
}

pub fn erase_sector(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_flash() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    const ADDR: u32 = 0x0200;
    let pat: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
    if wr(&dev, ADDR, &pat) != 0 {
        return Verdict::Fail("页编程失败");
    }
    if dev.ioctl(FLASH_IOCTL_ERASE_SECTOR, (&ADDR as *const u32 as *mut u32).cast()) != 0 {
        return Verdict::Fail("扇区擦除失败");
    }
    let mut back = [0xFFu8; 4];
    if rd(&dev, ADDR, &mut back) != 0 {
        return Verdict::Fail("READ 失败");
    }
    if back != [0xFF; 4] {
        return Verdict::Fail("擦除后未读回全 0xFF");
    }
    info!("spi_flash", "扇区擦除后读回全 0xFF");
    Verdict::Pass("擦除生效")
}

/// 持久化标记：固定地址写特征串。模拟器验收在宿主机侧调用
/// `machine.persist_spi_slaves()` 后校验文件映像含该串（数据跨 run 存活）。
pub fn persist_marker(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_flash() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    if wr(&dev, MARKER_ADDR, MARKER) != 0 {
        return Verdict::Fail("持久化标记写入失败");
    }
    // 读回确认已落盘（内存映像）
    let mut back = [0u8; MARKER.len()];
    if rd(&dev, MARKER_ADDR, &mut back) != 0 || &back != MARKER {
        return Verdict::Fail("持久化标记读回不一致");
    }
    info!("spi_flash", "持久化标记 @0x{MARKER_ADDR:03X} 已写入");
    Verdict::Pass("标记已写入")
}
