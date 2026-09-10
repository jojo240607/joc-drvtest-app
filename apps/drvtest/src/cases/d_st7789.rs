//! d_st7789：ST7789 LCD 虚拟器件（FSMC 8080 并行接口，Bank1 NE1）全链路。
//! 判据（与模拟器 vperiph/fsmc/st7789.rs 同一协议）：
//!   初始化序列后 RDDID 读回 0x85（ST7789）；窗口填充 RGB565 写显存
//!   （显存内容由模拟器侧断言）。

use rtos_app_sdk::device::Device;
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{LcdFill, LCD_IOCTL_FILL, LCD_IOCTL_GET_ID, LCD_IOCTL_INIT};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

fn open_lcd() -> Result<Device, &'static str> {
    cap::open("lcd0")
}

pub fn init_and_id(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_lcd() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    if dev.ioctl(LCD_IOCTL_INIT, core::ptr::null_mut()) != 0 {
        return Verdict::Fail("INIT 失败");
    }
    let mut id: u8 = 0;
    if dev.ioctl(LCD_IOCTL_GET_ID, (&mut id as *mut u8).cast()) != 0 {
        return Verdict::Fail("GET_ID 失败");
    }
    if id != 0x85 {
        return Verdict::Fail("RDDID 不符");
    }
    info!("lcd", "ST7789 RDDID=0x{id:02X} 正确");
    Verdict::Pass("ST7789 RDDID=0x85")
}

pub fn fill_window(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_lcd() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    if dev.ioctl(LCD_IOCTL_INIT, core::ptr::null_mut()) != 0 {
        return Verdict::Fail("INIT 失败");
    }
    // 顶部 2 行填充红色（模拟器侧断言 pixel(0,0) 等）
    let mut f = LcdFill { x0: 0, y0: 0, x1: 239, y1: 1, color: 0xF800 };
    if dev.ioctl(LCD_IOCTL_FILL, (&mut f as *mut LcdFill).cast()) != 0 {
        return Verdict::Fail("FILL 失败");
    }
    info!("lcd", "窗口 (0,0)-(239,1) 已填充 0xF800");
    Verdict::Pass("ST7789 窗口填充完成（显存模拟器侧断言）")
}
