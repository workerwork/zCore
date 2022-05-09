//! Uart device driver.

mod buffered;
mod uart_16550;

#[cfg(feature = "board-d1")]
mod uart_allwinner;

pub use buffered::BufferedUart;
pub use uart_16550::Uart16550Mmio;

#[cfg(target_arch = "x86_64")]
pub use uart_16550::Uart16550Pmio;

#[cfg(feature = "board-d1")]
pub use uart_allwinner::UartAllwinner;
