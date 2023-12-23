#[cfg(target_arch = "wasm32")]
use super::WEB_SERIAL_API_SUPPORTED;

use super::{PlotPage, SplotApp, TimeUnit};
use crate::serialconnection::{DataBits, FlowControl, Parity, StopBits};

impl SplotApp {
    pub fn draw_ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .open(&mut self.show_about_window)
            .collapsible(false)
            .auto_sized()
            .show(ctx, |ui| {
                ui.set_width(300.0);

                ui.vertical_centered_justified(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../misc/splot_logo.svg"))
                            .max_size(egui::Vec2 { x: 256., y: 256. }),
                    );

                    ui.label(egui::RichText::new("- Splot -").heading());
                    ui.add_space(12.0);
                    ui.label("A multi-platform serial plotter and monitor");
                    ui.separator();
                    ui.label("Created by:");
                    ui.label("Felix Zwettler");
                    ui.hyperlink_to("Repository", "https://github.com/flxzt/splot");
                    ui.separator();
                    ui.label("Powered by:");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui/");
                    ui.hyperlink_to("rust", "https://www.rust-lang.org/");
                });
            });

        egui::Window::new("Usage")
            .open(&mut self.show_usage_window)
            .collapsible(false)
            .auto_sized()
            .show(ctx, |ui| {
                ui.set_width(500.0);

                ui.vertical(|ui| {
                    ui.label(
"Splot parses data coming from a serial connection and looks for values separated by the specified separator and terminated by a newline character.
Each value is inserted by its index, so it is important to keep the number of values per line constant."
);

                ui.add_space(12.0);
                ui.label("Example:");
                ui.code("UART_Transmit(\"%i, %i\\n\", var_1, var_2);");

                ui.add_space(12.0);
                ui.label(
"Values can also have a name, which will appear in the values list. To specify a name, prefix the variable with \"<name>=\""
);

                ui.add_space(12.0);
                ui.label("Example:");
                ui.code("UART_Transmit(\"dist=%i, temperature=%i\\n\", var_1, var_2);");

                ui.add_space(12.0);
                ui.label(
"A special named value is the one with \"time=\" or \"t=\".
This indicates that this value should used as the time for plotting.
It must be monotonically increasing, so probably should come from a timer.
The time unit for the time values received by the device can be set in the application.
If no such variable is specified, the application takes the time when receiving the data"
);

                ui.add_space(12.0);
                ui.label("Example:");
                ui.code("UART_Transmit(\"time=%i, %i, %i\\n\", HAL_GetTick(), var_1, var_2);");
                });
            });

        egui::Window::new("Help")
            .open(&mut self.show_help_window)
            .collapsible(false)
            .auto_sized()
            .show(ctx, |ui| {
                ui.set_width(500.0);

                ui.vertical(|ui| {
                    ui.label(
                        "- Issue: When using the app on the web, the port does not show up.
    Fix: Restart the browser. The device might need to be plugged in before the browser is launched.
",
                    )
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                self.render_top_bar(ui, ctx);

                ui.separator();

                // Controls
                self.render_connection_controls(ui, ctx);

                ui.add_space(5.0);

                // Plots
                ui.group(|ui| {
                    ui.centered_and_justified(|ui| match self.plot_page {
                        PlotPage::TimeValue => self.render_plot_tv(ui),
                        PlotPage::XY => self.render_plot_xy(ui),
                        PlotPage::SerialMonitor => self.render_serial_monitor(ui),
                    });
                });
            });
        });
    }

    #[allow(unused)]
    fn render_top_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.menu_button("Splot", |ui| {
                if ui.button("About").clicked() {
                    ui.close_menu();
                    self.show_about_window = true;
                }

                #[cfg(not(target_arch = "wasm32"))] // no close() on web pages!
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                }
            });

            if ui.button("Usage").clicked() {
                self.show_usage_window = true;
            }

            if ui.button("Help").clicked() {
                self.show_help_window = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);

                #[cfg(feature = "demo")]
                {
                    ui.add(egui::Label::new(
                        egui::RichText::new("Demo Mode").color(egui::Color32::DARK_GREEN),
                    ));
                }

                #[cfg(not(feature = "demo"))]
                if ui
                    .toggle_value(&mut self.dummy_connection, "Dummy connection")
                    .changed()
                {
                    self.reset_connection(ctx);
                }
                ui.label(format!("Received Samples: {}", self.samples_received));

                egui::warn_if_debug_build(ui);

                #[cfg(target_arch = "wasm32")]
                {
                    #[cfg(not(feature = "demo"))]
                    let cond = !*WEB_SERIAL_API_SUPPORTED && !self.dummy_connection;

                    #[cfg(feature = "demo")]
                    let cond = false;

                    if cond {
                        ui.label(
                            egui::RichText::new(
                                "⚠ Web Serial API not supported ⚠\n on this platform ",
                            )
                            .small()
                            .color(egui::Color32::RED),
                        );
                    }
                }
            });
        });
    }

    fn render_connection_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered_justified(|ui| {
            ui.horizontal(|ui| {
                ui.label("Port: ");

                if egui::ComboBox::new("available_ports_combobox", "")
                    .selected_text(
                        self.selected_port_index
                            .and_then(|i| self.available_ports.get(i).map(|s| s.as_str()))
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for (i, available_port) in self.available_ports.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_port_index,
                                Some(i),
                                available_port,
                            );
                        }
                    })
                    .response
                    .clicked()
                {
                    self.available_ports(ctx);
                }

                if ui.button("⟲").clicked() {
                    self.available_ports(ctx);
                }

                ui.label("Baudrate: ");
                ui.add(egui::DragValue::new(&mut self.baudrate));

                ui.label("Data Bits:");
                egui::ComboBox::from_id_source("data_bits_combobox")
                    .selected_text(self.data_bits.to_string())
                    .width(30.0)
                    .show_ui(ui, |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        ui.selectable_value(
                            &mut self.data_bits,
                            DataBits::Five,
                            DataBits::Five.to_string(),
                        );
                        #[cfg(not(target_arch = "wasm32"))]
                        ui.selectable_value(
                            &mut self.data_bits,
                            DataBits::Six,
                            DataBits::Six.to_string(),
                        );
                        ui.selectable_value(
                            &mut self.data_bits,
                            DataBits::Seven,
                            DataBits::Seven.to_string(),
                        );
                        ui.selectable_value(
                            &mut self.data_bits,
                            DataBits::Eight,
                            DataBits::Eight.to_string(),
                        );
                    });

                ui.label("Flow Control:");
                egui::ComboBox::from_id_source("flow_control_combobox")
                    .selected_text(self.flow_control.to_string())
                    .width(30.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.flow_control,
                            FlowControl::None,
                            FlowControl::None.to_string(),
                        );
                        #[cfg(not(target_arch = "wasm32"))]
                        ui.selectable_value(
                            &mut self.flow_control,
                            FlowControl::Software,
                            FlowControl::Software.to_string(),
                        );
                        ui.selectable_value(
                            &mut self.flow_control,
                            FlowControl::Hardware,
                            FlowControl::Hardware.to_string(),
                        );
                    });

                ui.label("Parity:");
                egui::ComboBox::from_id_source("parity_combobox")
                    .selected_text(self.parity.to_string())
                    .width(30.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.parity,
                            Parity::None,
                            Parity::None.to_string(),
                        );
                        ui.selectable_value(&mut self.parity, Parity::Odd, Parity::Odd.to_string());
                        ui.selectable_value(
                            &mut self.parity,
                            Parity::Even,
                            Parity::Even.to_string(),
                        );
                    });

                ui.label("Stop Bits:");
                egui::ComboBox::from_id_source("stop_bits_combobox")
                    .selected_text(self.stop_bits.to_string())
                    .width(30.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.stop_bits,
                            StopBits::One,
                            StopBits::One.to_string(),
                        );
                        ui.selectable_value(
                            &mut self.stop_bits,
                            StopBits::Two,
                            StopBits::Two.to_string(),
                        );
                    });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button = egui::Button::new("Connect");

                    #[cfg(target_arch = "wasm32")]
                    let button_resp = {
                        #[cfg(not(feature = "demo"))]
                        let cond = *WEB_SERIAL_API_SUPPORTED || self.dummy_connection;

                        #[cfg(feature = "demo")]
                        let cond = true;

                        ui.add_enabled(cond, button)
                    };

                    #[cfg(not(target_arch = "wasm32"))]
                    let button_resp = ui.add(button);

                    if button_resp.clicked() {
                        self.try_connect(ctx);
                    }

                    ui.separator();
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Pages: ");
                ui.selectable_value(
                    &mut self.plot_page,
                    PlotPage::TimeValue,
                    PlotPage::TimeValue.to_string(),
                );
                ui.selectable_value(&mut self.plot_page, PlotPage::XY, PlotPage::XY.to_string());
                ui.selectable_value(
                    &mut self.plot_page,
                    PlotPage::SerialMonitor,
                    PlotPage::SerialMonitor.to_string(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset").clicked() {
                        self.reset_connection(ctx);
                    }

                    if ui.button("Clear").clicked() {
                        self.clear_samples(ctx);
                    }

                    ui.toggle_value(&mut self.pause, "Pause");

                    ui.separator();

                    let comboxbox_response = egui::ComboBox::from_id_source("time_unit_combobox")
                        .selected_text(self.time_unit.to_string())
                        .width(30.0)
                        .show_ui(ui, |ui| {
                            let mut changed = false;

                            changed |= ui
                                .selectable_value(
                                    &mut self.time_unit,
                                    TimeUnit::Us,
                                    TimeUnit::Us.to_string(),
                                )
                                .changed();
                            changed |= ui
                                .selectable_value(
                                    &mut self.time_unit,
                                    TimeUnit::Ms,
                                    TimeUnit::Ms.to_string(),
                                )
                                .changed();
                            changed |= ui
                                .selectable_value(
                                    &mut self.time_unit,
                                    TimeUnit::S,
                                    TimeUnit::S.to_string(),
                                )
                                .changed();

                            changed
                        });

                    if comboxbox_response.inner.unwrap_or(false) {
                        log::debug!("time unit has changed. clearing samples");
                        self.clear_samples(ctx);
                    }
                    ui.label("Time Unit: ");

                    egui::ComboBox::from_id_source("value_separator_combobox")
                        .selected_text(self.value_separator.to_string())
                        .width(30.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.value_separator, ',', ",");
                            ui.selectable_value(&mut self.value_separator, ';', ";");
                            ui.selectable_value(&mut self.value_separator, ':', ":");
                        });
                    ui.label("Value Separator: ");

                    ui.separator();
                });
            });
        });
    }

    fn render_plot_tv(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            egui::ScrollArea::vertical()
                .id_source("plot_scroll_area")
                .show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                        |ui| {
                            ui.set_width(270.0);

                            ui.horizontal(|ui| {
                                ui.label("Values newer:");
                                ui.add(
                                    egui::Slider::new(&mut self.plot_tv_newer, 0.1..=500.0)
                                        .logarithmic(true)
                                        .suffix(TimeUnit::S.to_string()),
                                );
                            });

                            ui.add_space(5.0);

                            for i in 0..self.samples_appearance.len() {
                                ui.group(|ui| {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Min),
                                        |ui| {
                                            ui.horizontal(|ui| {
                                                egui::color_picker::color_edit_button_rgba(
                                                    ui,
                                                    &mut self.samples_appearance[i].color,
                                                    egui::widgets::color_picker::Alpha::Opaque,
                                                );
                                                ui.checkbox(
                                                    &mut self.samples_appearance[i].visible,
                                                    "",
                                                );
                                                ui.text_edit_singleline(
                                                    &mut self.samples_appearance[i].name,
                                                );
                                            });
                                        },
                                    )
                                });

                                ui.end_row();
                            }
                        },
                    );
                });

            ui.separator();

            egui_plot::Plot::new("plot_tv")
                .label_formatter(move |name, value| {
                    if !name.is_empty() {
                        format!(
                            "{}\nt: {} {}\nv: {}",
                            name,
                            round_to_decimals(value.x, 7),
                            TimeUnit::S,
                            round_to_decimals(value.y, 7),
                        )
                    } else {
                        format!(
                            "t: {} {}\nv: {}",
                            round_to_decimals(value.x, 7),
                            TimeUnit::S,
                            round_to_decimals(value.y, 7),
                        )
                    }
                })
                .x_axis_formatter(move |val, _c, _range| {
                    format!("{} {}", round_to_decimals(val, 5), TimeUnit::S)
                })
                .y_axis_formatter(move |val, _c, _range| round_to_decimals(val, 7).to_string())
                .allow_zoom(egui::Vec2b { x: false, y: true })
                .allow_boxed_zoom(false)
                .show(ui, |plot_ui| {
                    for (i, samples) in self.samples_vec.iter().enumerate() {
                        if !self.samples_appearance[i].visible {
                            continue;
                        }

                        let Some(first) = self.samples_vec.first().and_then(|b| b.first()) else {
                            continue;
                        };

                        let Some(last) = self.samples_vec.first().and_then(|b| b.last()) else {
                            continue;
                        };

                        let last_plot_bounds = plot_ui.plot_bounds();
                        let plot_bounds = egui_plot::PlotBounds::from_min_max(
                            [last.time - self.plot_tv_newer, last_plot_bounds.min()[1]],
                            [last.time, last_plot_bounds.max()[1]],
                        );
                        plot_ui.set_plot_bounds(plot_bounds);

                        let plot_line = egui_plot::Line::new(
                            samples
                                .into_iter()
                                .filter_map(|s| {
                                    if last.time - s.time < self.plot_tv_newer {
                                        Some([s.time, s.value])
                                    } else {
                                        None
                                    }
                                })
                                .collect::<egui_plot::PlotPoints>(),
                        )
                        .name(&self.samples_appearance[i].name)
                        .color(self.samples_appearance[i].color);

                        let start_vline_val = first.time.max(last.time - self.plot_tv_newer);

                        plot_ui.vline(
                            egui_plot::VLine::new(start_vline_val)
                                .style(egui_plot::LineStyle::Dashed { length: 2.0 })
                                .color(egui::Color32::LIGHT_BLUE),
                        );

                        plot_ui.line(plot_line);
                    }
                });
        });
    }

    fn render_plot_xy(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            egui::Grid::new("plot_xy_grid").show(ui, |ui| {
                ui.set_width(270.0);

                ui.label("Values newer:");
                ui.add(
                    egui::Slider::new(&mut self.plot_xy_newer, 0.1..=500.0)
                        .logarithmic(true)
                        .suffix(TimeUnit::S.to_string()),
                );
                ui.end_row();

                ui.label("X-Axis");
                egui::ComboBox::from_id_source("samples_x_combobox")
                    .selected_text(
                        self.samples_appearance
                            .get(self.plot_xy_samples_x)
                            .map(|s| s.name.as_str())
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for i in 0..self.samples_vec.len() {
                            ui.selectable_value(
                                &mut self.plot_xy_samples_x,
                                i,
                                &self.samples_appearance[i].name,
                            );
                        }
                    });
                ui.end_row();

                ui.label("Y-Axis");
                egui::ComboBox::from_id_source("samples_y_combobox")
                    .selected_text(
                        self.samples_appearance
                            .get(self.plot_xy_samples_y)
                            .map(|s| s.name.as_str())
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for i in 0..self.samples_vec.len() {
                            ui.selectable_value(
                                &mut self.plot_xy_samples_y,
                                i,
                                &self.samples_appearance[i].name,
                            );
                        }
                    });
                ui.end_row();
            });

            ui.separator();

            egui_plot::Plot::new("xy plot")
                .x_axis_formatter(move |val, _c, _range| round_to_decimals(val, 7).to_string())
                .y_axis_formatter(move |val, _c, _range| round_to_decimals(val, 7).to_string())
                .show(ui, |plot_ui| {
                    if let (Some(samples_x), Some(samples_y)) = (
                        self.samples_vec.get(self.plot_xy_samples_x),
                        self.samples_vec.get(self.plot_xy_samples_y),
                    ) {
                        if let (Some(last_x), Some(last_y)) = (samples_x.last(), samples_y.last()) {
                            let plot_line = egui_plot::Line::new(
                                samples_x
                                    .into_iter()
                                    .zip(samples_y)
                                    .filter_map(|(x, y)| {
                                        if last_x.time - x.time < self.plot_xy_newer {
                                            Some([x.value, y.value])
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<egui_plot::PlotPoints>(),
                            )
                            .color(egui::Color32::DARK_RED);
                            let last_point =
                                egui_plot::Points::new(vec![[last_x.value, last_y.value]])
                                    .color(egui::Color32::RED)
                                    .highlight(true);

                            plot_ui.line(plot_line);
                            plot_ui.points(last_point);
                        }
                    }
                });
        });
    }

    fn render_serial_monitor(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_source("serial_monitor_scroll_area")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let monitor_text: String = self
                    .serial_monitor_lines
                    .iter()
                    .fold(String::new(), |acc, x| acc + x);

                ui.text_edit_multiline(&mut monitor_text.as_str());
            });
    }
}

/// Round a value to the given number of decimal places.
///
/// Taken from egui::emath
pub fn round_to_decimals(value: f64, decimal_places: usize) -> f64 {
    // This is a stupid way of doing this, but stupid works.
    format!("{value:.decimal_places$}").parse().unwrap_or(value)
}
