#![allow(dead_code)]

use crate::{
    scheme::{impl_event_scheme, Scheme, UartScheme},
    utils::EventListener,
    DeviceResult, VirtAddr,
};
use spin::Mutex;

use crate::builder::IoMapper;
use d1_pac::uart;
use d1_pac::{UART0, UART1, UART2, UART3, UART4, UART5};

pub struct UartAllwinner {
    inner: Mutex<Inner>,
    listener: EventListener,
}

impl_event_scheme!(UartAllwinner);

impl UartAllwinner {
    pub fn new(io_mapper: impl IoMapper, uart: &str) -> Self {
        let reg_size = core::mem::size_of::<uart::RegisterBlock>();
        let uart: usize = match uart {
            "uart0" => io_mapper.query_or_map(UART0::PTR as _, reg_size).unwrap(),
            "uart1" => io_mapper.query_or_map(UART1::PTR as _, reg_size).unwrap(),
            "uart2" => io_mapper.query_or_map(UART2::PTR as _, reg_size).unwrap(),
            "uart3" => io_mapper.query_or_map(UART3::PTR as _, reg_size).unwrap(),
            "uart4" => io_mapper.query_or_map(UART4::PTR as _, reg_size).unwrap(),
            "uart5" => io_mapper.query_or_map(UART5::PTR as _, reg_size).unwrap(),
            _ => {
                unimplemented!();
            }
        };
        let inner = Inner(uart);
        inner.init();
        Self {
            inner: Mutex::new(inner),
            listener: EventListener::new(),
        }
    }
}

impl Scheme for UartAllwinner {
    fn name(&self) -> &str {
        "uart-allwinner"
    }

    fn handle_irq(&self, _irq_num: usize) {
        self.listener.trigger(());
    }
}

impl UartScheme for UartAllwinner {
    fn try_recv(&self) -> DeviceResult<Option<u8>> {
        self.inner.lock().try_recv()
    }

    fn send(&self, ch: u8) -> DeviceResult {
        self.inner.lock().send(ch)
    }

    fn write_str(&self, s: &str) -> DeviceResult {
        self.inner.lock().write_str(s)
    }
}

struct Inner(VirtAddr);

impl Inner {
    /// initializes uart controller
    /// BAUD 115200
    /// FIFO ON
    fn init(&self) {
        let uart = self.uart();
        // disable interrupts
        uart.ier().reset();

        // enable fifo
        uart.fcr().write(|w| w.fifoe().set_bit());
        {
            uart.halt.write(|w| w.halt_tx().set_bit());
            uart.lcr.write(|w| w.dlab().set_bit());
            // 13 for 115200
            uart.dll().write(unsafe { |w| w.dll().bits(13) });
            uart.dlh().write(unsafe { |w| w.dlh().bits(0) });
            uart.lcr.write(|w| w.dlab().clear_bit());
            uart.halt.write(|w| {
                w.halt_tx()
                    .clear_bit()
                    .chcfg_at_busy()
                    .set_bit()
                    .change_update()
                    .set_bit()
            });
        }
        // no break | parity disabled | 1 stop bit | 8 data bits
        uart.lcr.write(|w| w.dls().eight());
        // reset fifo
        uart.fcr()
            .write(|w| w.xfifor().set_bit().rfifor().set_bit());
        // uart mode
        uart.mcr.write(unsafe { |w| w.bits(0x00) });
        // enable interrupts
        uart.ier().write(|w| w.erbfi().set_bit());
    }

    /// recives
    fn try_recv(&self) -> DeviceResult<Option<u8>> {
        if self.uart().lsr.read().dr().is_ready() {
            // u32 -> u8 大到小强转，语义上不安全，但实际逻辑上是安全的
            Ok(Some(self.uart().rbr().read().rbr().bits() as u8))
        } else {
            Ok(None)
        }
    }

    /// send
    fn send(&self, ch: u8) -> DeviceResult {
        // query mode
        while !self.uart().lsr.read().thre().is_empty() {}
        self.uart()
            .thr()
            .write(unsafe { |w| w.thr().bits(ch as _) });
        Ok(())
    }

    /// write_str
    fn write_str(&self, s: &str) -> DeviceResult {
        for b in s.bytes() {
            match b {
                b'\n' => {
                    self.send(b'\r')?;
                    self.send(b'\n')?;
                }
                _ => self.send(b)?,
            }
        }
        Ok(())
    }

    /// Converts a `usize` to a `mutable reference`
    #[inline]
    fn uart(&self) -> &mut uart::RegisterBlock {
        unsafe { &mut *(self.0 as *mut uart::RegisterBlock) }
    }
}
