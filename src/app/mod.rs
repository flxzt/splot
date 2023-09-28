pub mod ui;

use futures::lock::Mutex;
use instant::{Duration, Instant};
use std::collections::VecDeque;
use std::io::{BufRead, Cursor};
use std::sync::Arc;

use crate::fixedsizebuffer::FixedSizeBuffer;
#[allow(unused)]
use crate::serialconnection::new_serial_connection;
use crate::serialconnection::{
    new_serial_connection_dummy, DataBits, FlowControl, Parity, SerialConnection, StopBits,
};

#[derive(Debug, Clone)]
pub struct Sample {
    time: f64,
    value: f64,
    name: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
const SAMPLES_BUF_SIZE: usize = 16384;
#[cfg(target_arch = "wasm32")]
const SAMPLES_BUF_SIZE: usize = 2048;
#[cfg(not(target_arch = "wasm32"))]
const MONITOR_LINES_BUF_SIZE: usize = 512;
#[cfg(target_arch = "wasm32")]
const MONITOR_LINES_BUF_SIZE: usize = 128;

const READ_BUF_SIZE: usize = 32;

impl From<Sample> for egui_plot::PlotPoint {
    fn from(sample: Sample) -> Self {
        egui_plot::PlotPoint {
            x: sample.time,
            y: sample.value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseResult {
    full_lines: Vec<String>,
    /// Outer vec is one for each position, inner vec is the "history"
    samples_vec: Vec<Vec<Sample>>,
    n_new_samples: u64,
}

/// reads full lines and counts the number of read bytes
fn read_full_lines(input_buf: &[u8]) -> std::io::Result<(Vec<String>, usize)> {
    let mut lines = vec![];
    let mut read_bytes = 0;

    let mut line = String::new();
    let mut input_cursor = Cursor::new(input_buf);
    loop {
        let b = match input_cursor.read_line(&mut line) {
            // Continue if not valid UTF-8 (or some other error)
            Err(_e) => continue,
            // if 0, the last line terminates with EOF, so is not a full line
            Ok(0) => break,
            Ok(b) => b,
        };

        // detect unfinished lines
        if !line.ends_with('\n') {
            break;
        }

        lines.push(std::mem::take(&mut line));
        read_bytes += b;
    }

    Ok((lines, read_bytes))
}

#[derive(Debug, Clone, Default)]
pub struct Parser {
    buf: Vec<u8>,
}

impl Parser {
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn parse_from_serial_data(
        &mut self,
        serial_data: &[u8],
        time_unit: TimeUnit,
        value_separator: char,
        start_time: Instant,
    ) -> anyhow::Result<ParseResult> {
        self.buf.extend(serial_data);

        let mut added_samples = 0;
        let mut samples_vec: Vec<Vec<Sample>> = vec![];

        let mut time = Instant::now().duration_since(start_time).as_secs_f64();

        // Read out full lines
        let (full_lines, bytes_read) = read_full_lines(&self.buf)?;

        // Drain the buffer by the bytes length of the read full lines
        self.buf.drain(..bytes_read);

        // parse them
        for line in full_lines.iter() {
            let line = line.trim();

            // Don't add empy lines
            if line.is_empty() {
                continue;
            }

            for (i, value_str) in line.split(value_separator).enumerate() {
                let mut is_time = false;

                let mut name_splits: VecDeque<&str> =
                    value_str.split('=').map(|s| s.trim()).collect();

                let name = if name_splits.len() > 1 {
                    let name = name_splits.pop_front();

                    if let Some(name) = name {
                        is_time = name == "time" || name == "t";
                    }

                    name
                } else {
                    None
                };

                let Some(value) = name_splits.pop_front().and_then(|s| {
                    s.chars()
                        .filter(|&c| c.is_ascii_digit() || c == '-' || c == '.')
                        .collect::<String>()
                        .parse()
                        .ok()
                }) else {
                    continue;
                };

                if is_time {
                    time = time_unit.convert_to_secs(value);
                    continue;
                }

                added_samples += 1;

                if let Some(samples) = samples_vec.get_mut(i) {
                    samples.push(Sample {
                        time,
                        value,
                        name: name.map(|s| s.to_string()),
                    })
                } else {
                    samples_vec.push(vec![Sample {
                        time,
                        value,
                        name: name.map(|s| s.to_string()),
                    }]);
                }
            }
        }

        Ok(ParseResult {
            full_lines,
            samples_vec,
            n_new_samples: added_samples,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SamplesAppearance {
    name: String,
    visible: bool,
    color: egui::Rgba,
}

impl SamplesAppearance {
    fn new(name: String) -> Self {
        Self {
            name,
            visible: true,
            color: egui::Rgba::BLUE,
        }
    }
}

fn unique_color_in_list(i: usize, len: usize) -> egui::Rgba {
    let hue = i as f32 / len as f32;

    egui::ecolor::Hsva::new(hue, 0.8, 0.95, 1.0).into()
}

fn recolor_samples_appearances(appereances: &mut [SamplesAppearance]) {
    let len = appereances.len();

    for (i, a) in appereances.iter_mut().enumerate() {
        a.color = unique_color_in_list(i, len);
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum TimeUnit {
    Us,
    Ms,
    S,
}

impl Default for TimeUnit {
    fn default() -> Self {
        Self::S
    }
}

impl std::fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeUnit::Us => write!(f, "us"),
            TimeUnit::Ms => write!(f, "ms"),
            TimeUnit::S => write!(f, "s"),
        }
    }
}

impl TimeUnit {
    #[allow(unused)]
    fn convert_from_secs(self, secs: f64) -> f64 {
        match self {
            TimeUnit::Us => secs * 1_000_000.0,
            TimeUnit::Ms => secs * 1000.0,
            TimeUnit::S => secs,
        }
    }

    fn convert_to_secs(self, val: f64) -> f64 {
        match self {
            TimeUnit::Us => val / 1_000_000.0,
            TimeUnit::Ms => val / 1000.0,
            TimeUnit::S => val,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlotPage {
    TimeValue,
    XY,
    SerialMonitor,
}

impl Default for PlotPage {
    fn default() -> Self {
        Self::TimeValue
    }
}

impl std::fmt::Display for PlotPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlotPage::TimeValue => write!(f, "Time - Value"),
            PlotPage::XY => write!(f, "X - Y"),
            PlotPage::SerialMonitor => write!(f, "Serial Monitor"),
        }
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SplotApp {
    /// The baudrate
    baudrate: u32,
    /// The connection timeout
    timeout: Duration,
    /// Data bits
    data_bits: DataBits,
    /// Flow control
    flow_control: FlowControl,
    /// Parity
    parity: Parity,
    /// Stop bits
    stop_bits: StopBits,

    /// The unit used for received time values
    time_unit: TimeUnit,
    /// The value separator
    value_separator: char,
    /// if the dummy connection should be used
    /// ( not available with demo feature, there the dummy connection is always used )
    #[cfg(not(feature = "demo"))]
    dummy_connection: bool,

    #[serde(skip)]
    serial_connection: Arc<Mutex<Box<dyn SerialConnection>>>,
    #[serde(skip)]
    start_time: Instant,
    #[serde(skip)]
    samples_vec: Vec<FixedSizeBuffer<Sample>>,
    #[serde(skip)]
    samples_received: u64,
    /// The parser has internal state
    #[serde(skip)]
    parser: Parser,
    /// pause reading the serial connection
    #[serde(skip)]
    pause: bool,

    // Ui state
    #[serde(skip)]
    show_about_window: bool,
    #[serde(skip)]
    show_usage_window: bool,
    #[serde(skip)]
    show_help_window: bool,
    #[serde(skip)]
    selected_port_index: Option<usize>,
    #[serde(skip)]
    serial_monitor_lines: FixedSizeBuffer<String>,
    #[serde(skip)]
    samples_appearance: Vec<SamplesAppearance>,
    #[serde(skip)]
    plot_page: PlotPage,
    /// Only display measurements this far back
    #[serde(skip)]
    plot_tv_newer: f64,
    #[serde(skip)]
    plot_tv_bounds: egui_plot::PlotBounds,

    #[serde(skip)]
    plot_xy_samples_x: usize,
    #[serde(skip)]
    plot_xy_samples_y: usize,
    /// Only display measurements this far back
    #[serde(skip)]
    plot_xy_newer: f64,

    // Async state
    #[serde(skip)]
    promise_available_ports: Option<poll_promise::Promise<Vec<String>>>,
    #[serde(skip)]
    promise_try_connect: Option<poll_promise::Promise<anyhow::Result<()>>>,
    #[serde(skip)]
    promise_read: Option<poll_promise::Promise<anyhow::Result<Vec<u8>>>>,
    #[serde(skip)]
    is_connected: bool,
    #[serde(skip)]
    available_ports: Vec<String>,
}

impl Default for SplotApp {
    fn default() -> Self {
        let serial_connection = Arc::new(Mutex::new(new_serial_connection_dummy()));
        let now = Instant::now();

        Self {
            baudrate: 115200,
            timeout: Duration::from_millis(5000),
            data_bits: DataBits::default(),
            flow_control: FlowControl::default(),
            parity: Parity::default(),
            stop_bits: StopBits::default(),

            time_unit: TimeUnit::default(),
            value_separator: ',',
            #[cfg(not(feature = "demo"))]
            dummy_connection: false,

            serial_connection,
            start_time: now,
            samples_vec: vec![],
            samples_received: 0,
            parser: Parser::default(),
            pause: false,

            show_about_window: false,
            show_usage_window: false,
            show_help_window: false,
            selected_port_index: None,
            serial_monitor_lines: FixedSizeBuffer::new(MONITOR_LINES_BUF_SIZE),
            samples_appearance: vec![],
            plot_page: PlotPage::default(),
            plot_tv_newer: 10.0,
            plot_tv_bounds: egui_plot::PlotBounds::NOTHING,

            plot_xy_samples_x: 0,
            plot_xy_samples_y: 0,
            plot_xy_newer: 10.0,

            promise_available_ports: None,
            promise_try_connect: None,
            promise_read: None,
            is_connected: false,
            available_ports: vec![],
        }
    }
}

impl SplotApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let mut app: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            app.setup(&cc.egui_ctx);
            return app;
        }

        let mut app = Self::default();
        app.setup(&cc.egui_ctx);

        app
    }

    /// Some things need to be set up at runtime
    pub fn setup(&mut self, ctx: &egui::Context) {
        self.reset_connection(ctx);
        egui_extras::install_image_loaders(ctx);
    }

    #[allow(unused)]
    pub fn clear_samples(&mut self, ctx: &egui::Context) {
        self.samples_received = 0;
        self.samples_vec.clear();
        self.samples_appearance.clear();
        self.serial_monitor_lines.clear();
    }

    pub fn reset_connection(&mut self, ctx: &egui::Context) {
        self.clear_samples(ctx);
        self.parser.clear();

        self.selected_port_index.take();
        self.available_ports.clear();
        self.plot_xy_samples_x = 0;
        self.plot_xy_samples_y = 0;

        self.promise_available_ports.take();
        self.promise_try_connect.take();
        self.promise_read.take();

        #[cfg(feature = "demo")]
        {
            // Always the dummy connection as demo
            self.serial_connection = Arc::new(Mutex::new(new_serial_connection_dummy()));
        }

        #[cfg(not(feature = "demo"))]
        if self.dummy_connection {
            self.serial_connection = Arc::new(Mutex::new(new_serial_connection_dummy()));
        } else {
            self.serial_connection = Arc::new(Mutex::new(new_serial_connection()));
        }

        // Start listing available ports
        self.available_ports(ctx);
        // And start reading
        self.read(ctx);
    }

    /// Installs the available_ports promise and polls for its readiness
    fn available_ports(&mut self, ctx: &egui::Context) {
        let c = Arc::clone(&self.serial_connection);

        let _ = self.promise_available_ports.get_or_insert_with(move || {
            poll_promise::Promise::spawn_local(
                async move { c.lock().await.available_ports().await },
            )
        });

        self.poll_available_ports(ctx);
    }

    /// Installs the try_connect promise and polls for its readiness
    pub fn try_connect(&mut self, ctx: &egui::Context) {
        let c = Arc::clone(&self.serial_connection);

        if let Some(selected_port_index) = self.selected_port_index {
            let baudrate = self.baudrate;
            let timeout = self.timeout;
            let data_bits = self.data_bits;
            let flow_control = self.flow_control;
            let parity = self.parity;
            let stop_bits = self.stop_bits;

            // try connect
            let _ = self.promise_try_connect.get_or_insert_with(|| {
                poll_promise::Promise::spawn_local(async move {
                    c.lock()
                        .await
                        .try_connect(
                            selected_port_index,
                            baudrate,
                            timeout,
                            data_bits,
                            flow_control,
                            parity,
                            stop_bits,
                        )
                        .await
                })
            });

            self.poll_try_connect(ctx);
        }
    }

    /// Installs the read promise and polls for its readiness
    fn read(&mut self, ctx: &egui::Context) {
        let c = Arc::clone(&self.serial_connection);

        // read from serial port
        let _ = self.promise_read.get_or_insert_with(move || {
            poll_promise::Promise::spawn_local(async move {
                if c.lock().await.is_connected() {
                    c.lock().await.read(READ_BUF_SIZE).await
                } else {
                    Ok(vec![])
                }
            })
        });

        self.poll_read(ctx);
    }

    fn poll_available_ports(&mut self, ctx: &egui::Context) {
        let Some(promise_available_ports) = self.promise_available_ports.as_mut() else {
            return;
        };

        if let Some(available_ports) = promise_available_ports.ready() {
            self.available_ports = available_ports.clone();

            self.promise_available_ports.take();
            ctx.request_repaint();
        }
    }

    fn poll_try_connect(&mut self, ctx: &egui::Context) {
        let Some(promise_try_connect) = self.promise_try_connect.as_mut() else {
            return;
        };

        if let Some(res) = promise_try_connect.ready() {
            if let Err(e) = res {
                log::error!("try_connect() failed, Err: {}", e);
            } else {
                self.start_time = Instant::now();
            }

            self.promise_try_connect.take();

            ctx.request_repaint();
        }
    }

    fn poll_read(&mut self, ctx: &egui::Context) {
        let Some(promise_read) = self.promise_read.as_mut() else {
            return;
        };

        if let Some(data_res) = promise_read.ready() {
            match data_res {
                Ok(serial_data) => {
                    match self.parser.parse_from_serial_data(
                        serial_data,
                        self.time_unit,
                        self.value_separator,
                        self.start_time,
                    ) {
                        Ok(res) => {
                            if !res.full_lines.is_empty() {
                                self.serial_monitor_lines.extend(res.full_lines);
                            }

                            if res.n_new_samples > 0 {
                                for (i, new_samples) in res.samples_vec.into_iter().enumerate() {
                                    if let Some(samples) = self.samples_vec.get_mut(i) {
                                        samples.extend(new_samples);
                                    } else {
                                        // Grow samples vec

                                        // Give it the name of the first sample if provided
                                        let name = new_samples
                                            .first()
                                            .and_then(|sample| sample.name.clone());

                                        let mut new_buf = FixedSizeBuffer::new(SAMPLES_BUF_SIZE);
                                        new_buf.extend(new_samples);

                                        self.samples_vec.push(new_buf);

                                        self.samples_appearance.push(SamplesAppearance::new(
                                            name.unwrap_or_else(|| format!("Samples {i:02}")),
                                        ));

                                        recolor_samples_appearances(&mut self.samples_appearance);
                                    }
                                }

                                self.samples_received += res.n_new_samples;
                            }
                        }
                        Err(e) => {
                            log::debug!("failed to add samples from serial data, Err: `{e}`");
                            self.parser.clear();
                        }
                    }
                }
                Err(e) => log::warn!("device read failed, Err: `{e}`"),
            }

            self.promise_read.take();

            // Always install another read
            self.read(ctx);
        }
    }

    /// Needs to be called repeatedly to poll promises
    pub fn async_tasks(&mut self, ctx: &egui::Context) {
        self.poll_available_ports(ctx);
        self.poll_try_connect(ctx);

        if !self.pause {
            self.poll_read(ctx);
        }

        #[cfg(not(target_arch = "wasm32"))]
        poll_promise::tick_local();
    }
}

impl eframe::App for SplotApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.async_tasks(ctx);

        self.draw_ui(ctx, frame);

        // repaint periodically
        ctx.request_repaint_after(instant::Duration::from_secs_f64(1.0 / 60.0));
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) static WEB_SERIAL_API_SUPPORTED: once_cell::sync::Lazy<bool> =
    once_cell::sync::Lazy::new(|| {
        let serial_itf = web_sys::window().unwrap().navigator().serial();

        !serial_itf.is_undefined()
    });
