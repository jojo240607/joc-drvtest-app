//! 能力探测：统一「设备缺失 / open 失败」→ Skip 决策。
//!
//! 用例的通用开场白：
//! ```ignore
//! let dev = match cap::open("adc0") { Ok(d) => d, Err(r) => return Verdict::Skip(r) };
//! ```
//! 这样同一套用例在模拟器 / 真机上都能跑：设备不在 → Skip；open 失败 → Skip。
//!
//! 模拟器已知阻塞器的处理原则（不在此枚举，而是在用例内设计绕开）：
//! - uart1/2/3 的 DMA TX 在模拟器上可能永久阻塞 → 用例只测 open + ioctl，不写；
//! - adc0 默认 IRQ 引擎依赖外部注入 → 用例先 SET_MODE(POLL)（SWSTART 即时完成）；
//! - spi xfer / i2c 主从传输需从机应答 → 用例只测扫描/寄存器回读，不传输。
//! 真机上这些路径正常，用例同一份代码直接跑（判据更严的数值检查在真机同样成立）。

use rtos_app_sdk::device::Device;

/// 按名查找设备（不 open）。`Err` 携带 Skip 原因。
pub fn get(name: &str) -> Result<Device, &'static str> {
    Device::get(name).ok_or("no such device")
}

/// 按名查找并 open 设备。`Err` 携带 Skip 原因（区分「不存在」与「open 失败」）。
pub fn open(name: &str) -> Result<Device, &'static str> {
    if Device::get(name).is_none() {
        return Err("no such device");
    }
    match Device::open(name) {
        Some(d) => Ok(d),
        None => Err("open failed"),
    }
}
