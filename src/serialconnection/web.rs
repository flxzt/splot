use async_trait::async_trait;
use instant::Duration;

use super::{DataBits, FlowControl, Parity, SerialConnection, StopBits};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// The port name to request a port from the user.
const REQUEST_PORT_STR: &str = "Request port";
/// Indicate that the Web Serial API is not supported
const WEB_SERIAL_UNSUPPORTED_STR: &str = "Web Serial API is unsupported by this platform.";

impl From<Parity> for web_sys::ParityType {
    fn from(v: Parity) -> Self {
        match v {
            Parity::None => Self::None,
            Parity::Odd => Self::Odd,
            Parity::Even => Self::Even,
        }
    }
}

impl TryFrom<FlowControl> for web_sys::FlowControlType {
    type Error = anyhow::Error;

    fn try_from(v: FlowControl) -> Result<Self, Self::Error> {
        match v {
            FlowControl::None => Ok(Self::None),
            FlowControl::Software => Err(anyhow::anyhow!(
                "FlowControl Software not supported in Web Serial API"
            )),
            FlowControl::Hardware => Ok(Self::Hardware),
        }
    }
}
#[derive(Debug)]
pub struct SerialConnectionWeb {
    /// the port, and if it is opened
    requested_ports: Vec<web_sys::SerialPort>,
    active_port: Option<usize>,
}

#[async_trait(?Send)]
impl SerialConnection for SerialConnectionWeb {
    async fn available_ports(&mut self) -> Vec<String> {
        if !check_serial_api_supported() {
            log::warn!("{WEB_SERIAL_UNSUPPORTED_STR}");
            return vec![];
        }

        let serial_itf = web_sys::window().unwrap().navigator().serial();

        let mut available_ports_ret = vec![REQUEST_PORT_STR.to_string()];
        self.requested_ports.clear();

        if let Ok(ports) = JsFuture::from(serial_itf.get_ports()).await {
            if let Ok(Some(ports_iter)) = js_sys::try_iter(&ports) {
                for (i, port) in ports_iter.enumerate() {
                    if let Ok(port) = port {
                        let port = web_sys::SerialPort::from(port);
                        log::debug!("got port: {port:?}");

                        let info = port.get_info();
                        log::debug!("got port info: {info:?}");

                        // Try to get PID and VID, but sometimes this is undefined (At least on chrome, linux)
                        if let (Ok(vid), Ok(pid)) = (
                            js_sys::Reflect::get(
                                &info,
                                &wasm_bindgen::JsValue::from("usbVendorId"),
                            ),
                            js_sys::Reflect::get(
                                &info,
                                &wasm_bindgen::JsValue::from("usbProductId"),
                            ),
                        ) {
                            if !vid.is_undefined() && !pid.is_undefined() {
                                log::debug!("got port info - device pid: {:?} vid: {:?}", vid, pid);
                            }
                        }

                        available_ports_ret.push(format!("port `{i}`"));
                        self.requested_ports.push(port);
                    }
                }
            }
        }

        available_ports_ret
    }

    async fn try_connect(
        &mut self,
        port_index: usize,
        baudrate: u32,
        _timeout: Duration,
        data_bits: DataBits,
        flow_control: FlowControl,
        parity: Parity,
        stop_bits: StopBits,
    ) -> anyhow::Result<()> {
        log::debug!("try_connect() with port index: '{port_index}'");

        if !check_serial_api_supported() {
            return Err(anyhow::anyhow!(
                "serial connection try_connect() aborted, web serial API not supported."
            ));
        }

        let serial_itf = web_sys::window().unwrap().navigator().serial();

        // always close first
        self.close_all_ports().await?;

        // first is always request port
        if port_index == 0 {
            if let Ok(port) = JsFuture::from(serial_itf.request_port()).await {
                let port = web_sys::SerialPort::from(port);
                let info = port.get_info();

                log::debug!("{info:?}");
            }

            return Ok(());
        }

        if let Some(serial_port) = self.requested_ports.get_mut(port_index - 1) {
            let options =
                create_web_serial_options(baudrate, data_bits, flow_control, parity, stop_bits)?;

            if let Err(_e) = JsFuture::from(serial_port.open(&options)).await {
                log::warn!("port was already opened.");
            };

            self.active_port = Some(port_index - 1);
        } else {
            return Err(anyhow::anyhow!("no port available for index: {port_index}"));
        }

        Ok(())
    }

    fn is_connected(&mut self) -> bool {
        if !check_serial_api_supported() {
            return false;
        }

        self.active_port.is_some()
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        self.active_port = None;
        self.close_all_ports().await?;
        self.requested_ports.clear();
        Ok(())
    }

    async fn read(&mut self, _read_buf_size: usize) -> anyhow::Result<Vec<u8>> {
        if !check_serial_api_supported() {
            return Err(anyhow::anyhow!(
                "serial connection read() aborted, web serial API not supported."
            ));
        }

        if let Some(port) = self.active_port.and_then(|a| self.requested_ports.get(a)) {
            let readable = port.readable();

            if readable.is_null() {
                log::warn!("can't read from port. readable is null.");
                return Ok(vec![]);
            }

            let reader = readable
                .get_reader()
                .dyn_into::<web_sys::ReadableStreamDefaultReader>()
                .map_err(|e| {
                    anyhow::anyhow!(
                        "failed to cast reader into ReadableStreamDefaultHandler, Err {e:?}"
                    )
                })?;
            let read_data = JsFuture::from(reader.read())
                .await
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;

            let data = js_sys::Reflect::get(&read_data, &JsValue::from("value"))
                .and_then(|jsv| jsv.dyn_into::<js_sys::Uint8Array>())
                .map_err(|e| anyhow::anyhow!("{e:?}"))?
                .to_vec();

            reader.release_lock();

            return Ok(data);
        }

        Ok(vec![])
    }
}

impl SerialConnectionWeb {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            requested_ports: vec![],
            active_port: None,
        }
    }

    async fn close_all_ports(&mut self) -> anyhow::Result<()> {
        for (i, port) in self.requested_ports.iter().enumerate() {
            if let Err(_e) = JsFuture::from(port.close()).await {
                log::warn!("close_all_ports(), port {i} was already closed.");
            }
        }

        Ok(())
    }
}

fn check_serial_api_supported() -> bool {
    let serial_itf = web_sys::window().unwrap().navigator().serial();

    !serial_itf.is_undefined()
}

fn create_web_serial_options(
    baudrate: u32,
    data_bits: DataBits,
    flow_control: FlowControl,
    parity: Parity,
    stop_bits: StopBits,
) -> anyhow::Result<web_sys::SerialOptions> {
    let mut options = web_sys::SerialOptions::new(baudrate);
    let o_data_bits = match data_bits {
        data_bits @ DataBits::Five | data_bits @ DataBits::Six => {
            return Err(anyhow::anyhow!(
                "Data Bits: {data_bits} not supported in Web Serial API"
            ))
        }
        DataBits::Seven => 7,
        DataBits::Eight => 8,
    };

    options.data_bits(o_data_bits);
    options.parity(parity.into());
    options.flow_control(web_sys::FlowControlType::try_from(flow_control)?);

    let o_stop_bits = match stop_bits {
        StopBits::One => 1,
        StopBits::Two => 2,
    };
    options.stop_bits(o_stop_bits);

    Ok(options)
}
