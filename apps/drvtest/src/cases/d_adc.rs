//! d_adc：ADC + 内部温度传感器。
//! adc0 默认 IRQ 引擎在模拟器上依赖外部注入可能阻塞 → 用例先 SET_MODE(POLL)
//!（SWSTART 即时完成，读取不阻塞；真机 POLL 同样成立）。

use rtos_app_sdk::ioctl::*;
use rtos_app_sdk::rtos::msleep;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// adc0：通道设置/回读 + POLL 单次转换读取（12 位值域校验）。
pub fn adc0_read(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("adc0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 切 POLL 引擎（模拟器 IRQ/DMA 依赖外部注入；POLL 由 SWSTART 即时置 EOC）。
    let mut mode: u32 = STREAM_MODE_POLL;
    if dev.ioctl(STREAM_IOCTL_SET_MODE, (&mut mode as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_MODE(POLL) 失败");
    }

    // 通道设置/回读。
    let mut ch: u32 = 0;
    if dev.ioctl(ADC_IOCTL_SET_CHANNEL, (&mut ch as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_CHANNEL 失败");
    }
    let mut gch: u32 = 0xFF;
    if dev.ioctl(ADC_IOCTL_GET_CHANNEL, (&mut gch as *mut u32).cast()) != 0 || gch != 0 {
        return Verdict::Fail("GET_CHANNEL 与设置不一致");
    }

    // 单次转换读取：SWSTART → EOC → DR（12 位原始码）。
    let mut raw = [0u8; 4];
    let rc = dev.read(&mut raw);
    if rc != 4 {
        return Verdict::Fail("adc read 未返回 u32");
    }
    let v = u32::from_le_bytes(raw);
    if v > 4095 {
        return Verdict::Fail("12 位转换值越界");
    }

    // 再次转换：值应仍为合法 12 位码（且两次路径一致）。
    msleep(5);
    let mut raw2 = [0u8; 4];
    let rc2 = dev.read(&mut raw2);
    if rc2 != 4 || u32::from_le_bytes(raw2) > 4095 {
        return Verdict::Fail("二次转换读取异常");
    }

    Verdict::Pass("adc0 POLL 单次转换可读（12 位值域合法）")
}

/// temp0：读回内部温度（float 摄氏度，合理范围 -40..125）。
pub fn temp0_read(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("temp0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut raw = [0u8; 4];
    let rc = dev.read(&mut raw);
    if rc != 4 {
        return Verdict::Fail("temp read 未返回 float");
    }
    let t = f32::from_le_bytes(raw);
    if t.is_nan() || t < -40.0 || t > 125.0 {
        return Verdict::Fail("温度读数越界");
    }
    Verdict::Pass("temp0 温度读数合法")
}
