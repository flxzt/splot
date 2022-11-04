use async_trait::async_trait;
use instant::Duration;

pub mod dummy;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}

impl Default for DataBits {
    fn default() -> Self {
        Self::Eight
    }
}

impl std::fmt::Display for DataBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataBits::Five => write!(f, "Five"),
            DataBits::Six => write!(f, "Six"),
            DataBits::Seven => write!(f, "Seven"),
            DataBits::Eight => write!(f, "Eight"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum FlowControl {
    /// No flow control.
    None,
    /// Flow control using XON/XOFF bytes.
    Software,
    /// Flow control using RTS/CTS signals.
    Hardware,
}

impl Default for FlowControl {
    fn default() -> Self {
        Self::None
    }
}

impl std::fmt::Display for FlowControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowControl::None => write!(f, "None"),
            FlowControl::Software => write!(f, "Software"),
            FlowControl::Hardware => write!(f, "Hardware"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl Default for Parity {
    fn default() -> Self {
        Self::None
    }
}

impl std::fmt::Display for Parity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Parity::None => write!(f, "None"),
            Parity::Odd => write!(f, "Odd"),
            Parity::Even => write!(f, "Even"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum StopBits {
    One,
    Two,
}

impl Default for StopBits {
    fn default() -> Self {
        Self::One
    }
}

impl std::fmt::Display for StopBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopBits::One => write!(f, "One"),
            StopBits::Two => write!(f, "Two"),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn new_serial_connection() -> Box<dyn SerialConnection> {
    Box::new(web::SerialConnectionWeb::new())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn new_serial_connection() -> Box<dyn SerialConnection> {
    Box::new(native::SerialConnectionNative::new())
}

pub fn new_serial_connection_dummy() -> Box<dyn SerialConnection> {
    Box::new(dummy::SerialConnectionDummy::new())
}

#[async_trait(?Send)]
pub trait SerialConnection {
    async fn available_ports(&mut self) -> Vec<String>;

    /// The port index must with the item index of the vector returned by `available_ports()`.
    #[allow(clippy::too_many_arguments)]
    async fn try_connect(
        &mut self,
        port_index: usize,
        baudrate: u32,
        timeout: Duration,
        data_bits: DataBits,
        flow_control: FlowControl,
        parity: Parity,
        stop_bits: StopBits,
    ) -> anyhow::Result<()>;

    fn is_connected(&mut self) -> bool;

    async fn close(&mut self) -> anyhow::Result<()>;

    async fn read(&mut self, read_buf_size: usize) -> anyhow::Result<Vec<u8>>;
}
