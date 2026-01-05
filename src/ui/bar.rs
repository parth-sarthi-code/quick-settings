use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Box, Button, CenterBox, Label, Orientation};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::RwLock;

use crate::state::AppState;
use crate::ui::widgets;
use crate::ui::quick_settings::QuickSettings;
use crate::services::device_client::DeviceClient;

pub struct TopBar {
    window: ApplicationWindow,
    workspace_label: Label,
    window_title_label: Label,
    clock_label: Label,
    battery_label: Label,
    wifi_label: Label,
    bluetooth_label: Label,
    warp_label: Label,
    #[allow(dead_code)]
    quick_settings: Arc<QuickSettings>,
}

impl TopBar {
    pub fn new(app: &Application, state: Arc<RwLock<AppState>>) -> Self {
        // Create main window
        let window = ApplicationWindow::builder()
            .application(app)
            .build();

        // Initialize layer shell for the window
        window.init_layer_shell();
        
        // Configure as a top panel
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        
        // Set exclusive zone (bar height)
        window.set_exclusive_zone(32);
        
        // Set keyboard mode to none (don't grab keyboard)
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        // Create main horizontal container
        let main_box = CenterBox::builder()
            .margin_start(8)
            .margin_end(8)
            .margin_top(4)
            .margin_bottom(4)
            .build();

        // Left section: workspace + window title
        let left_box = Box::new(Orientation::Horizontal, 12);
        
        let workspace_label = Label::builder()
            .label("—")
            .css_classes(vec!["workspace-indicator"])
            .build();
        
        let window_title_label = Label::builder()
            .label("")
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(60)
            .css_classes(vec!["window-title"])
            .build();
        
        left_box.append(&workspace_label);
        left_box.append(&window_title_label);

        // Center section: clock
        let clock_label = Label::builder()
            .label(widgets::current_time_string().as_str())
            .css_classes(vec!["clock"])
            .build();

        // Right section: system status + quick settings button
        let right_box = Box::new(Orientation::Horizontal, 6);
        
        // Battery indicator
        let battery_label = Label::builder()
            .label("� --")
            .css_classes(vec!["status-indicator"])
            .build();
        right_box.append(&battery_label);
        
        // WiFi indicator
        let wifi_label = Label::builder()
            .label("󰤭")
            .css_classes(vec!["status-indicator"])
            .build();
        right_box.append(&wifi_label);
        
        // Bluetooth indicator
        let bluetooth_label = Label::builder()
            .label("󰂲")
            .css_classes(vec!["status-indicator"])
            .build();
        right_box.append(&bluetooth_label);
        
        // Warp indicator (visible when connected)
        let warp_label = Label::builder()
            .label("")
            .css_classes(vec!["status-indicator", "warp-icon"])
            .build();
        right_box.append(&warp_label);
        
        let settings_btn = Button::with_label("⚙");
        settings_btn.set_css_classes(&["quick-settings-btn"]);
        right_box.append(&settings_btn);

        // Add sections to main box
        main_box.set_start_widget(Some(&left_box));
        main_box.set_center_widget(Some(&clock_label));
        main_box.set_end_widget(Some(&right_box));

        // Add CSS styling
        let css_provider = gtk4::CssProvider::new();
        css_provider.load_from_data(
            "window {\
                background-color: rgba(30, 30, 30, 0.95);\
                color: rgba(255, 255, 255, 1);\
                font-size: 13px;\
            }\
            \
            .workspace-indicator {\
                font-weight: bold;\
                padding: 0 8px;\
                background-color: rgba(70, 130, 180, 0.3);\
                border-radius: 4px;\
            }\
            \
            .window-title {\
                color: rgba(204, 204, 204, 1);\
            }\
            \
            .clock {\
                font-weight: 500;\
            }\
            \
            .quick-settings-btn {\
                padding: 4px 8px;\
                border-radius: 4px;\
                background-color: rgba(70, 70, 70, 0.5);\
                color: rgba(255, 255, 255, 0.8);\
                border: none;\
            }\
            \
            .quick-settings-btn:hover {\
                background-color: rgba(100, 100, 100, 0.7);\
                color: rgba(255, 255, 255, 1);\
            }\
            \
            .status-indicator {\
                font-size: 15px;\
                color: rgba(200, 200, 200, 0.9);\
                padding: 0 5px;\
            }\
            \
            .warp-icon {\
                color: rgba(100, 200, 255, 1.0);\
                font-weight: bold;\
            }"
        );

        gtk4::style_context_add_provider_for_display(
            &gtk4::prelude::WidgetExt::display(&window),
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        window.set_child(Some(&main_box));

        // Create quick settings panel
        let quick_settings = QuickSettings::new(app, Arc::clone(&state));
        
        // Connect quick settings button
        let quick_settings_clone = Arc::clone(&quick_settings);
        settings_btn.connect_clicked(move |_| {
            let qs = Arc::clone(&quick_settings_clone);
            glib::spawn_future_local(async move {
                qs.toggle().await;
            });
        });

        let bar = Self {
            window,
            workspace_label,
            window_title_label,
            clock_label,
            battery_label,
            wifi_label,
            bluetooth_label,
            warp_label,
            quick_settings,
        };

        // Start update loops
        bar.start_clock_updates();
        bar.start_state_updates(state);
        bar.start_status_updates();

        bar
    }

    pub fn show(&self) {
        self.window.present();
    }

    /// Update clock every minute
    fn start_clock_updates(&self) {
        let clock_label = self.clock_label.clone();
        
        glib::timeout_add_seconds_local(60, move || {
            clock_label.set_text(&widgets::current_time_string());
            glib::ControlFlow::Continue
        });
    }

    /// Poll state and update UI
    /// In production, this should be event-driven, not polled
    fn start_state_updates(&self, state: Arc<RwLock<AppState>>) {
        let workspace_label = self.workspace_label.clone();
        let window_title_label = self.window_title_label.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let state = state.clone();
            let workspace_label = workspace_label.clone();
            let window_title_label = window_title_label.clone();
            
            glib::spawn_future_local(async move {
                let app_state = state.read().await;
                workspace_label.set_text(&app_state.workspace_text());
                window_title_label.set_text(&app_state.window_title());
            });

            glib::ControlFlow::Continue
        });
    }

    /// Update battery, WiFi, Bluetooth, Warp status (every 5s with caching)
    fn start_status_updates(&self) {
        let battery_label = self.battery_label.clone();
        let wifi_label = self.wifi_label.clone();
        let bluetooth_label = self.bluetooth_label.clone();
        let warp_label = self.warp_label.clone();

        // Cache previous values to avoid redundant updates
        let prev_battery = Rc::new(RefCell::new(String::new()));
        let prev_wifi = Rc::new(RefCell::new(String::new()));
        let prev_bluetooth = Rc::new(RefCell::new(String::new()));
        let prev_warp = Rc::new(RefCell::new(String::new()));

        glib::timeout_add_seconds_local(5, move || {
            let battery_label = battery_label.clone();
            let wifi_label = wifi_label.clone();
            let bluetooth_label = bluetooth_label.clone();
            let warp_label = warp_label.clone();
            let prev_battery = Rc::clone(&prev_battery);
            let prev_wifi = Rc::clone(&prev_wifi);
            let prev_bluetooth = Rc::clone(&prev_bluetooth);
            let prev_warp = Rc::clone(&prev_warp);

            glib::spawn_future_local(async move {
                // Battery
                if let Ok(output) = tokio::process::Command::new("upower")
                    .args(["-i", "/org/freedesktop/UPower/devices/battery_BAT0"])
                    .output()
                    .await
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(pct_line) = stdout.lines().find(|l| l.contains("percentage:")) {
                        if let Some(pct_str) = pct_line.split(':').nth(1) {
                            if let Ok(pct) = pct_str.trim().trim_end_matches('%').parse::<u32>() {
                                let text = match pct {
                                    80..=100 => format!("󰁹 {}%", pct),
                                    50..=79 => format!("󰂀 {}%", pct),
                                    20..=49 => format!("󰂂 {}%", pct),
                                    _ => format!("󰂃 {}%", pct),
                                };
                                let mut cached = prev_battery.borrow_mut();
                                if *cached != text {
                                    battery_label.set_label(&text);
                                    *cached = text;
                                }
                            }
                        }
                    }
                }

                // WiFi
                let client = DeviceClient::default();
                if let Ok(networks) = client.wifi_connected().await {
                    let wifi_text = if let Some(net) = networks {
                        format!("󰤨 {}", net.ssid)
                    } else {
                        "󰤨 —".to_string()
                    };
                    let mut cached = prev_wifi.borrow_mut();
                    if *cached != wifi_text {
                        wifi_label.set_label(&wifi_text);
                        *cached = wifi_text;
                    }
                }

                // Bluetooth
                if let Ok(Some(device)) = client.bluetooth_connected().await {
                    let bt_text = format!("󰂯 {}", device.name);
                    let mut cached = prev_bluetooth.borrow_mut();
                    if *cached != bt_text {
                        bluetooth_label.set_label(&bt_text);
                        *cached = bt_text;
                    }
                } else {
                    let mut cached = prev_bluetooth.borrow_mut();
                    if *cached != "󰂯" {
                        bluetooth_label.set_label("󰂯");
                        *cached = "󰂯".to_string();
                    }
                }

                // Warp
                if let Ok(output) = tokio::process::Command::new("warp-cli")
                    .arg("status")
                    .output()
                    .await
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let warp_text = if stdout.contains("Connected") {
                        "󱘖".to_string()
                    } else {
                        String::new()
                    };
                    let mut cached = prev_warp.borrow_mut();
                    if *cached != warp_text {
                        warp_label.set_label(&warp_text);
                        *cached = warp_text;
                    }
                }
            });

            glib::ControlFlow::Continue
        });
    }
}
