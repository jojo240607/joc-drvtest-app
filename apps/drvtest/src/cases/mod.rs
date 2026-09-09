//! 驱动用例分组注册表：按依赖顺序排列（地基 → 基础外设 → 总线/引擎）。
//! 新增驱动用例：写一个 `cases/d_xxx.rs`，在此 `pub mod` + 追加 `Case` 项即可。

pub mod d_can;
pub mod d_flash;
pub mod d_fsmc;
pub mod d_i2s;
pub mod d_sd_card;
pub mod d_usb;
pub mod d_wdg;
pub mod d_adc;
pub mod d_crc;
pub mod d_dac;
pub mod d_dma;
pub mod d_dma_multi;
pub mod d_exti;
pub mod d_exti_multi;
pub mod d_gpio;
pub mod d_pwm;
pub mod d_pwm_multi;
pub mod d_rtc;
pub mod d_rng;
pub mod d_i2c_multi;
pub mod d_spi_i2c;
pub mod d_spi_flash;
pub mod d_bmi088;
pub mod d_timer;
pub mod d_timer_multi;
pub mod d_uart;
pub mod k_sdk;

use crate::runner::Case;

/// 全部用例（const 数组；顺序即执行顺序）。
pub static CASES: &[Case] = &[
    /* ---- k_sdk：SDK/RTOS 地基 ---- */
    Case { group: "k_sdk", name: "tick_msleep", budget_ms: 500, run: k_sdk::tick_and_msleep },
    Case { group: "k_sdk", name: "spawn_sem", budget_ms: 1000, run: k_sdk::spawn_and_sem },
    Case { group: "k_sdk", name: "mutex_roundtrip", budget_ms: 500, run: k_sdk::mutex_roundtrip },
    Case { group: "k_sdk", name: "slot_devtable", budget_ms: 500, run: k_sdk::slot_device_table },

    /* ---- d_uart：串口 ---- */
    Case { group: "d_uart", name: "uart0_dma_tx_real", budget_ms: 1500, run: d_uart::uart0_dma_tx_real },
    Case { group: "d_uart", name: "uart0_console", budget_ms: 1500, run: d_uart::uart0_console },
    Case { group: "d_uart", name: "uart_others", budget_ms: 1500, run: d_uart::uart_others },

    /* ---- d_gpio：引脚 ---- */
    Case { group: "d_gpio", name: "output_roundtrip", budget_ms: 1500, run: d_gpio::gpio_output_roundtrip },
    Case { group: "d_gpio", name: "input_probe", budget_ms: 1000, run: d_gpio::gpio_input_probe },

    /* ---- d_adc：ADC + 温度 ---- */
    Case { group: "d_adc", name: "adc0_read", budget_ms: 1500, run: d_adc::adc0_read },
    Case { group: "d_adc", name: "temp0_read", budget_ms: 1500, run: d_adc::temp0_read },

    /* ---- d_rng：随机数 ---- */
    Case { group: "d_rng", name: "rng_read", budget_ms: 1000, run: d_rng::rng_read },

    /* ---- d_crc：CRC32 引擎（数值正确） ---- */
    Case { group: "d_crc", name: "crc32_known", budget_ms: 1000, run: d_crc::crc32_known },

    /* ---- d_rtc：实时时钟 ---- */
    Case { group: "d_rtc", name: "rtc_time", budget_ms: 1500, run: d_rtc::rtc_time },

    /* ---- d_timer / d_exti：定时器 + 中断路径 ---- */
    Case { group: "d_timer", name: "timer_overflow", budget_ms: 1500, run: d_timer::timer_overflow },
    Case { group: "d_timer", name: "timer_app_isr", budget_ms: 2000, run: d_timer::timer_app_isr },
    Case { group: "d_timer", name: "timer_instances", budget_ms: 4000, run: d_timer_multi::timer_instances },
    Case { group: "d_exti", name: "exti_trigger", budget_ms: 1000, run: d_exti::exti_trigger },
    Case { group: "d_exti", name: "exti_instances", budget_ms: 3000, run: d_exti_multi::exti_instances },

    /* ---- d_pwm / d_dac：模拟输出 ---- */
    Case { group: "d_pwm", name: "pwm_ioctl", budget_ms: 1500, run: d_pwm::pwm_ioctl },
    Case { group: "d_pwm", name: "pwm_instances", budget_ms: 2000, run: d_pwm_multi::pwm_instances },
    Case { group: "d_dac", name: "dac_ioctl", budget_ms: 1000, run: d_dac::dac_ioctl },

    /* ---- d_spi_i2c / d_dma：总线与引擎 ---- */
    Case { group: "d_spi_i2c", name: "i2c_scan", budget_ms: 6000, run: d_spi_i2c::i2c_scan },
    Case { group: "d_spi_i2c", name: "spi_cr1", budget_ms: 1000, run: d_spi_i2c::spi_cr1 },
    /* ---- d_bmi088：SPI 双片选 IMU 全链路（模拟器虚拟从机回送真实数据） ---- */
    Case { group: "d_bmi088", name: "who_am_i", budget_ms: 2000, run: d_bmi088::who_am_i },
    /* ---- v3：SPI NOR Flash（保存/存储） ---- */
    Case { group: "d_spi_flash", name: "jedec_id", budget_ms: 1000, run: d_spi_flash::jedec_id },
    Case { group: "d_spi_flash", name: "write_read_back", budget_ms: 1500, run: d_spi_flash::write_read_back },
    Case { group: "d_spi_flash", name: "erase_sector", budget_ms: 1500, run: d_spi_flash::erase_sector },
    Case { group: "d_spi_flash", name: "persist_marker", budget_ms: 1000, run: d_spi_flash::persist_marker },

    Case { group: "d_bmi088", name: "raw_readout", budget_ms: 2000, run: d_bmi088::raw_readout },
    Case { group: "d_bmi088", name: "si_readout", budget_ms: 2000, run: d_bmi088::si_readout },
    Case { group: "d_bmi088", name: "raw_read_12b", budget_ms: 2000, run: d_bmi088::raw_read_12b },
    Case { group: "d_spi_i2c", name: "i2c_instances", budget_ms: 6000, run: d_i2c_multi::i2c_instances },
    Case { group: "d_dma", name: "dma_pool", budget_ms: 1000, run: d_dma::dma_pool },
    Case { group: "d_dma", name: "dma2_pool", budget_ms: 500, run: d_dma_multi::dma2_pool },

    /* ---- v2：CAN / FLASH / WDG / I2S / SDIO+SD 卡 / USB ---- */
    Case { group: "d_can", name: "can_loopback", budget_ms: 2000, run: d_can::can_loopback },
    Case { group: "d_flash", name: "flash_ioctl", budget_ms: 1000, run: d_flash::flash_ioctl },
    Case { group: "d_fsmc", name: "fsmc_ioctl", budget_ms: 1500, run: d_fsmc::fsmc_ioctl },
    Case { group: "d_wdg", name: "iwdg_config", budget_ms: 1000, run: d_wdg::iwdg_config },
    Case { group: "d_wdg", name: "wwdg_config", budget_ms: 1000, run: d_wdg::wwdg_config },
    Case { group: "d_i2s", name: "i2s_config", budget_ms: 1500, run: d_i2s::i2s_config },
    Case { group: "d_sd_card", name: "sdio_sd_init", budget_ms: 3000, run: d_sd_card::sdio_sd_init },
    Case { group: "d_sd_card", name: "block_rw", budget_ms: 3000, run: d_sd_card::block_rw },
    Case { group: "d_usb", name: "usb_ioctl", budget_ms: 3000, run: d_usb::usb_ioctl },
    Case { group: "d_usb", name: "usb_host_comms", budget_ms: 6000, run: d_usb::usb_host_comms },
];
