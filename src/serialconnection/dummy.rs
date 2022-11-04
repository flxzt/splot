use async_trait::async_trait;
use instant::{Duration, Instant};

use super::{DataBits, FlowControl, Parity, SerialConnection, StopBits};

#[derive(Debug)]
pub struct SerialConnectionDummy {
    connected: bool,
    start_time: Instant,
    last_read: Instant,
}

/// The port name for the dummy device.
pub const DUMMY_PORT_STR: &str = "dummy";

#[async_trait(?Send)]
impl SerialConnection for SerialConnectionDummy {
    async fn available_ports(&mut self) -> Vec<String> {
        vec![DUMMY_PORT_STR.to_string()]
    }

    async fn try_connect(
        &mut self,
        port_index: usize,
        _baudrate: u32,
        _timeout: Duration,
        _data_bits: DataBits,
        _flow_control: FlowControl,
        _parity: Parity,
        _stop_bits: StopBits,
    ) -> anyhow::Result<()> {
        if port_index == 0 {
            let now = Instant::now();

            self.connected = true;
            self.start_time = now;
            self.last_read = now;

            Ok(())
        } else {
            self.connected = false;

            Err(anyhow::anyhow!(
                "failed to connect to dummy device. Invalid port index `{port_index}`"
            ))
        }
    }

    fn is_connected(&mut self) -> bool {
        self.connected
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn read(&mut self, _read_buf_size: usize) -> anyhow::Result<Vec<u8>> {
        if !self.connected {
            return Err(anyhow::anyhow!(
                "failed to read dummy serial port, not connected."
            ));
        }

        let now = Instant::now();
        let elapsed_since_start = now.duration_since(self.start_time).as_secs_f64();

        // Only emit values at this frequency
        let freq = 60.0;

        if now.duration_since(self.last_read).as_secs_f64() < 1.0 / freq {
            return Ok(vec![]);
        }

        let square_val = if elapsed_since_start.round() as u32 % 2 == 0 {
            0.2
        } else {
            -1.0
        };

        let sin_val = elapsed_since_start.sin() - 0.5;
        let sin2_val = (elapsed_since_start * 0.5).sin() * 0.7 + 0.3;

        let read_buf = format!(
            "time={elapsed_since_start:.4}, square={square_val:.4}, sin_1={sin_val:.4}, sin_2={sin2_val:.4} \n",
        )
        .into_bytes();

        self.last_read = now;

        Ok(read_buf)
    }
}

impl SerialConnectionDummy {
    #[allow(unused)]
    pub fn new() -> Self {
        let now = Instant::now();

        Self {
            connected: false,
            start_time: now,
            last_read: now,
        }
    }
}
