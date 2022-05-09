#![allow(dead_code)]

use crate::{
    io::{Io, Mmio},
    scheme::{impl_event_scheme, Scheme, UartScheme},
    utils::EventListener,
    DeviceResult, VirtAddr,
};
use spin::Mutex;

pub struct UartAllwinner {
    inner: Mutex<Inner>,
    listener: EventListener,
}

impl_event_scheme!(UartAllwinner);

impl UartAllwinner {
    pub fn new(base: VirtAddr) -> Self {
        let inner = Inner(base);
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

const RBR: usize = 0x00;
const THR: usize = 0x00;
const DLL: usize = 0x00;
const DLH: usize = 0x04;
const IER: usize = 0x04;
const IIR: usize = 0x08;
const FCR: usize = 0x08;
const LCR: usize = 0x0c;
const MCR: usize = 0x10;
const LSR: usize = 0x14;
const MSR: usize = 0x18;
const SCH: usize = 0x1c;
const USR: usize = 0x7c;
const TFL: usize = 0x80;
const RFL: usize = 0x84;
const HSK: usize = 0x88;
const DMA_REQ_EN: usize = 0x8c;
const HALT: usize = 0xa4;

impl Inner {
    /// 初始化串口控制器
    /// BAUD 115200
    /// FIFO ON
    fn init(&self) {
        // disable interrupts
        self.reg(IER).write(0);
        // enable fifo
        self.reg(FCR).write(0b0001);
        {
            self.reg(HALT).write(0b0000_0001);
            self.reg(LCR).write(0b1000_0011);
            // 13 for 115200
            self.reg(DLL).write(13);
            self.reg(DLH).write(0);
            // no break | parity disabled | 1 stop bit | 8 data bits
            self.reg(LCR).write(0b0000_0011);
            self.reg(HALT).write(0b0000_0110);
        }
        // reset fifo
        self.reg(FCR).write(0b0111);
        // uart mode
        self.reg(MCR).write(0);
        // enable interrupts
        self.reg(IER).write(1);
    }

    /// 接收
    fn try_recv(&self) -> DeviceResult<Option<u8>> {
        if (self.reg(LSR).read() & 1) != 0 {
            // u32 -> u8 大到小强转，语义上不安全，但实际逻辑上是安全的
            Ok(Some(self.reg(RBR).read() as u8))
        } else {
            Ok(None)
        }
    }

    /// 发送
    fn send(&self, ch: u8) -> DeviceResult {
        // query mode
        while (self.reg(LSR).read() & (1 << 5)) == 0 {}
        self.reg(THR).write(ch as _);
        Ok(())
    }

    /// 重写write_str方法覆盖默认实现
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

    /// 访问寄存器
    fn reg(&self, offset: usize) -> &mut Mmio<u32> {
        unsafe { Mmio::from_base(self.0 + offset) }
    }
}
