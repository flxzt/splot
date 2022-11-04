use async_trait::async_trait;
use instant::Duration;

use super::{DataBits, FlowControl, Parity, SerialConnection, StopBits};

impl From<DataBits> for serialport::DataBits {
    fn from(v: DataBits) -> Self {
        match v {
            DataBits::Five => Self::Five,
            DataBits::Six => Self::Six,
            DataBits::Seven => Self::Seven,
            DataBits::Eight => Self::Eight,
        }
    }
}

impl From<FlowControl> for serialport::FlowControl {
    fn from(v: FlowControl) -> Self {
        match v {
            FlowControl::None => Self::None,
            FlowControl::Software => Self::Software,
            FlowControl::Hardware => Self::Hardware,
        }
    }
}

impl From<Parity> for serialport::Parity {
    fn from(v: Parity) -> Self {
        match v {
            Parity::None => Self::None,
            Parity::Odd => Self::Odd,
            Parity::Even => Self::Even,
        }
    }
}

impl From<StopBits> for serialport::StopBits {
    fn from(v: StopBits) -> Self {
        match v {
            StopBits::One => Self::One,
            StopBits::Two => Self::Two,
        }
    }
}

pub struct SerialConnectionNative {
    port: Option<Box<dyn serialport::SerialPort>>,
    available_ports: Vec<serialport::SerialPortInfo>,
}

#[async_trait(?Send)]
impl SerialConnection for SerialConnectionNative {
    async fn available_ports(&mut self) -> Vec<String> {
        if let Ok(ports) = serialport::available_ports() {
            self.available_ports = ports.clone();
            ports.into_iter().map(|i| i.port_name).collect()
        } else {
            vec![]
        }
    }

    async fn try_connect(
        &mut self,
        port_index: usize,
        baudrate: u32,
        timeout: Duration,
        data_bits: DataBits,
        flow_control: FlowControl,
        parity: Parity,
        stop_bits: StopBits,
    ) -> anyhow::Result<()> {
        if let Some(port_info) = self.available_ports.get(port_index) {
            log::debug!("try_connect() to port '{}'", &port_info.port_name);

            // First drop the existing connection so that the port is not busy anymore
            if let Some(port) = self.port.take() {
                port.clear(serialport::ClearBuffer::All)?;
                drop(port);
            }

            let port = serialport::new(&port_info.port_name, baudrate)
                .timeout(timeout)
                .data_bits(data_bits.into())
                .flow_control(flow_control.into())
                .parity(parity.into())
                .stop_bits(stop_bits.into())
                .open()?;

            log::debug!("successfully connected to port: {}", &port_info.port_name);

            port.clear(serialport::ClearBuffer::All)?;

            self.port.replace(port);
        }
        Ok(())
    }

    fn is_connected(&mut self) -> bool {
        self.port.is_some()
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        self.port.take();
        Ok(())
    }

    async fn read(&mut self, read_buf_size: usize) -> anyhow::Result<Vec<u8>> {
        if let Some(port) = self.port.as_mut() {
            let mut read_buf = vec![0; read_buf_size];

            if port.bytes_to_read()? < read_buf_size as u32 {
                return Ok(vec![]);
            }
            let bytes_read = port.read(&mut read_buf)?;
            read_buf.resize(bytes_read, 0);

            Ok(read_buf)
        } else {
            Err(anyhow::anyhow!(
                "failed to read serial port, Not connected."
            ))
        }
    }
}

impl SerialConnectionNative {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            port: None,
            available_ports: vec![],
        }
    }
}
