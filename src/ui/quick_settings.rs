use gtk4::prelude::*;
use gtk4::{
    glib, Align, Application, ApplicationWindow, Box, Button, Grid, Label, ListBox, ListBoxRow,
    Orientation, Revealer, Scale, Stack,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::AudioOutput;
use crate::services::device_client::DeviceClient;
use crate::services::runtime;
use crate::state::AppState;
use crate::ui::bluetooth::BluetoothView;
use crate::ui::tiles;
use crate::ui::wifi::WifiView;

pub struct QuickSettings {
    window: ApplicationWindow,
    is_visible: Arc<RwLock<bool>>,
    stack: Stack,
    brightness_slider: Scale,
    volume_slider: Scale,
    volume_label: Label,
    outputs_revealer: Revealer,
    outputs_list: ListBox,
    outputs_toggle: Button,
    outputs: Rc<RefCell<Vec<AudioOutput>>>,
    device_client: Arc<DeviceClient>,
    wifi_tile: Button,
    wifi_tile_label: Label,
    wifi_tile_sublabel: Label,
    bt_tile: Button,
    bt_tile_label: Label,
    bt_tile_sublabel: Label,
    warp_tile: Button,
    warp_tile_label: Label,
    warp_tile_sublabel: Label,
    saver_btn: Button,
    balanced_btn: Button,
    perf_btn: Button,
    battery_label: Label,
    // Cached states to avoid redundant updates
    cached_wifi_status: Rc<RefCell<String>>,
    cached_bt_status: Rc<RefCell<String>>,
    cached_warp_status: Rc<RefCell<String>>,
}

impl QuickSettings {
    fn clone_for_update(&self) -> Self {
        Self {
            window: self.window.clone(),
            is_visible: Arc::clone(&self.is_visible),
            stack: self.stack.clone(),
            brightness_slider: self.brightness_slider.clone(),
            volume_slider: self.volume_slider.clone(),
            volume_label: self.volume_label.clone(),
            outputs_revealer: self.outputs_revealer.clone(),
            outputs_list: self.outputs_list.clone(),
            outputs_toggle: self.outputs_toggle.clone(),
            outputs: self.outputs.clone(),
            device_client: Arc::clone(&self.device_client),
            wifi_tile: self.wifi_tile.clone(),
            wifi_tile_label: self.wifi_tile_label.clone(),
            wifi_tile_sublabel: self.wifi_tile_sublabel.clone(),
            bt_tile: self.bt_tile.clone(),
            bt_tile_label: self.bt_tile_label.clone(),
            bt_tile_sublabel: self.bt_tile_sublabel.clone(),
            warp_tile: self.warp_tile.clone(),
            warp_tile_label: self.warp_tile_label.clone(),
            warp_tile_sublabel: self.warp_tile_sublabel.clone(),
            saver_btn: self.saver_btn.clone(),
            balanced_btn: self.balanced_btn.clone(),
            perf_btn: self.perf_btn.clone(),
            battery_label: self.battery_label.clone(),
            cached_wifi_status: self.cached_wifi_status.clone(),
            cached_bt_status: self.cached_bt_status.clone(),
            cached_warp_status: self.cached_warp_status.clone(),
        }
    }
}

impl QuickSettings {
    pub fn new(app: &Application, _state: Arc<RwLock<AppState>>) -> Arc<Self> {
        // Create a layer-shell window for the quick settings panel
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(300)
            .default_height(400)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 40);
        window.set_margin(Edge::Right, 8);
        window.set_keyboard_mode(KeyboardMode::OnDemand);

        // Stack for main and subviews
        let stack = Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .build();

        // Build main grid view
        let (
            main_root,
            brightness_slider,
            volume_slider,
            volume_label,
            outputs_toggle,
            outputs_revealer,
            outputs_list,
            wifi_btn,
            bt_btn,
            warp_btn,
            wifi_label,
            wifi_sublabel,
            bt_label,
            bt_sublabel,
            warp_label,
            warp_sublabel,
            saver_btn,
            balanced_btn,
            perf_btn,
            battery_label,
            header_power_btn,
        ) = Self::build_main_view();

        // Device client talks to the daemon; no D-Bus in the GTK process
        let device_client = Arc::new(DeviceClient::default());
        let outputs = Rc::new(RefCell::new(Vec::new()));

        // Build subviews
        let wifi_view = WifiView::new(Arc::clone(&device_client));
        let bt_view = BluetoothView::new(Arc::clone(&device_client));

        stack.add_named(&main_root, Some("main"));
        stack.add_named(&wifi_view.root, Some("wifi"));
        stack.add_named(&bt_view.root, Some("bt"));
        stack.set_visible_child_name("main");

        // Wire navigation
        {
            let stack = stack.clone();
            wifi_btn.connect_clicked(move |_| stack.set_visible_child_name("wifi"));
        }
        {
            let stack = stack.clone();
            bt_btn.connect_clicked(move |_| stack.set_visible_child_name("bt"));
        }
        {
            let stack = stack.clone();
            wifi_view.back_btn.connect_clicked(move |_| stack.set_visible_child_name("main"));
        }
        {
            let stack = stack.clone();
            bt_view.back_btn.connect_clicked(move |_| stack.set_visible_child_name("main"));
        }

        // Apply styling and set content
        Self::apply_styles(&window);
        window.set_child(Some(&stack));

        let quick_settings = Arc::new(Self {
            window,
            is_visible: Arc::new(RwLock::new(false)),
            stack,
            brightness_slider: brightness_slider.clone(),
            volume_slider: volume_slider.clone(),
            volume_label: volume_label.clone(),
            outputs_revealer: outputs_revealer.clone(),
            outputs_list: outputs_list.clone(),
            outputs_toggle: outputs_toggle.clone(),
            outputs: outputs.clone(),
            device_client: Arc::clone(&device_client),
            wifi_tile: wifi_btn,
            wifi_tile_label: wifi_label,
            wifi_tile_sublabel: wifi_sublabel,
            bt_tile: bt_btn,
            bt_tile_label: bt_label,
            bt_tile_sublabel: bt_sublabel,
            warp_tile: warp_btn,
            warp_tile_label: warp_label,
            warp_tile_sublabel: warp_sublabel,
            saver_btn,
            balanced_btn,
            perf_btn,
            battery_label: battery_label.clone(),
            cached_wifi_status: Rc::new(RefCell::new(String::new())),
            cached_bt_status: Rc::new(RefCell::new(String::new())),
            cached_warp_status: Rc::new(RefCell::new(String::new())),
        });

        // Setup power menu popover
        {
            let popover = gtk4::Popover::new();
            popover.set_parent(&header_power_btn);
            
            let menu_box = Box::new(Orientation::Vertical, 4);
            menu_box.set_margin_top(4);
            menu_box.set_margin_bottom(4);
            menu_box.set_margin_start(4);
            menu_box.set_margin_end(4);

            let logout_btn = Button::with_label("Logout");
            logout_btn.set_css_classes(&["power-menu-item"]);
            let shutdown_btn = Button::with_label("Shutdown");
            shutdown_btn.set_css_classes(&["power-menu-item"]);
            let reboot_btn = Button::with_label("Reboot");
            reboot_btn.set_css_classes(&["power-menu-item"]);

            menu_box.append(&logout_btn);
            menu_box.append(&shutdown_btn);
            menu_box.append(&reboot_btn);

            popover.set_child(Some(&menu_box));

            // Show popover on button click
            {
                let popover_clone = popover.clone();
                header_power_btn.connect_clicked(move |_| {
                    popover_clone.popup();
                });
            }

            // Handle menu actions
            {
                let popover_clone = popover.clone();
                logout_btn.connect_clicked(move |_| {
                    popover_clone.popdown();
                    let handle = runtime::handle();
                    handle.spawn(async move {
                        let _ = tokio::process::Command::new("loginctl")
                            .args(["terminate-session", "self"])
                            .output()
                            .await;
                    });
                });
            }

            {
                let popover_clone = popover.clone();
                shutdown_btn.connect_clicked(move |_| {
                    popover_clone.popdown();
                    let handle = runtime::handle();
                    handle.spawn(async move {
                        let _ = tokio::process::Command::new("systemctl")
                            .arg("poweroff")
                            .output()
                            .await;
                    });
                });
            }

            {
                let popover_clone = popover.clone();
                reboot_btn.connect_clicked(move |_| {
                    popover_clone.popdown();
                    let handle = runtime::handle();
                    handle.spawn(async move {
                        let _ = tokio::process::Command::new("systemctl")
                            .arg("reboot")
                            .output()
                            .await;
                    });
                });
            }
        }
        {
            let qs = Arc::clone(&quick_settings);
            quick_settings.warp_tile.connect_clicked(move |_| {
                let qs = Arc::clone(&qs);
                glib::spawn_future_local(async move {
                    qs.toggle_warp().await;
                });
            });
        }

        // Volume handler (stub - integrate with daemon if needed)
        {
            volume_slider.connect_value_changed(move |_s| {});
        }

        // Brightness handler - direct brightnessctl
        {
            brightness_slider.connect_value_changed(move |slider| {
                let percentage = slider.value().round() as u32;
                let handle = runtime::handle();
                handle.spawn(async move {
                    let _ = tokio::process::Command::new("brightnessctl")
                        .arg("set")
                        .arg(format!("{}%", percentage.min(100)))
                        .output()
                        .await;
                });
            });
        }

        // Load initial brightness
        {
            let brightness_slider_clone = brightness_slider.clone();
            let handle = runtime::handle();
            glib::spawn_future_local(async move {
                let pct_result = handle.spawn(async move {
                    if let Ok(output) = tokio::process::Command::new("brightnessctl").output().await {
                        if let Ok(stdout) = String::from_utf8(output.stdout) {
                            for line in stdout.lines() {
                                if line.contains("Current brightness:") {
                                    if let Some(pct_str) = line.split('(').nth(1) {
                                        if let Some(num_str) = pct_str.split('%').next() {
                                            if let Ok(pct) = num_str.trim().parse::<u32>() {
                                                return Some(pct);
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    None
                }).await;
                
                if let Ok(Some(pct)) = pct_result {
                    brightness_slider_clone.set_value(pct as f64);
                }
            });
        }

        // Volume handler - update label AND set volume when slider changes
        {
            let volume_label_clone = volume_label.clone();
            volume_slider.connect_value_changed(move |slider| {
                let value = slider.value() as u32;
                volume_label_clone.set_label(&format!("{}%", value));
                
                // Set volume via wpctl
                glib::spawn_future_local(async move {
                    if let Err(e) = set_volume_direct(value).await {
                        eprintln!("Volume set failed: {e}");
                    }
                });
            });
        }

        // Toggle output device list with smooth animation
        {
            let revealer = outputs_revealer.clone();
            let toggle = outputs_toggle.clone();
            outputs_toggle.connect_clicked(move |_| {
                let reveal = !revealer.reveals_child();
                revealer.set_reveal_child(reveal);
                toggle.set_label(if reveal { "^" } else { ">" });
            });
        }

        // Switch output device on selection
        {
            let qs = Arc::clone(&quick_settings);
            outputs_list.connect_row_activated(move |_lb, row| {
                let qs = Arc::clone(&qs);
                let idx = row.index();
                if idx >= 0 {
                    glib::spawn_future_local(async move {
                        let id = {
                            let outputs = qs.outputs.borrow();
                            outputs
                                .get(idx as usize)
                                .map(|o| o.id.clone())
                        };

                        if let Some(output_id) = id {
                            let handle = runtime::handle();
                            let slider = qs.volume_slider.clone();
                            let label = qs.volume_label.clone();
                            
                            let result = handle
                                .spawn({
                                    let client = Arc::clone(&qs.device_client);
                                    let output_id = output_id.clone();
                                    async move {
                                        // Set default device
                                        client.audio_set_default(&output_id).await?;
                                        
                                        // Small delay to let wpctl update
                                        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                                        
                                        // Fetch volume for the device
                                        match get_device_volume(&output_id).await {
                                            Ok(vol) => Ok(vol),
                                            Err(_) => {
                                                // Fallback to default sink
                                                get_default_volume().await
                                            }
                                        }
                                    }
                                })
                                .await;

                            match result {
                                Ok(Ok(vol)) => {
                                    slider.set_value(vol as f64);
                                    label.set_label(&format!("{}%", vol));
                                    qs.refresh_outputs_ui().await;
                                    qs.outputs_revealer.set_reveal_child(false);
                                    qs.outputs_toggle.set_label(">");
                                }
                                Ok(Err(e)) => eprintln!("Audio set default failed: {e}"),
                                Err(e) => eprintln!("Audio set default task failed: {e}"),
                            }
                        }
                    });
                }
            });
        }

        // Update WiFi status on show
        {
            let qs = Arc::clone(&quick_settings);
            glib::spawn_future_local(async move {
                qs.update_wifi_status().await;
                qs.update_bluetooth_status().await;
                qs.update_warp_status().await;
                qs.update_battery_status().await;
                qs.refresh_outputs_ui().await;
            });
        }

        // Periodically refresh status (reduced from 2s to 5s with caching)
        {
            let qs = Arc::clone(&quick_settings);
            glib::timeout_add_seconds_local(5, move || {
                let qs = Arc::clone(&qs);
                glib::spawn_future_local(async move {
                    qs.update_wifi_status().await;
                    qs.update_bluetooth_status().await;
                    qs.update_warp_status().await;
                    qs.update_battery_status().await;
                });
                glib::ControlFlow::Continue
            });
        }

        quick_settings
    }

    fn build_main_view() -> (
        Box,
        Scale,
        Scale,
        Label,
        Button,
        Revealer,
        ListBox,
        Button,
        Button,
        Button,
        Label,
        Label,
        Label,
        Label,
        Label,
        Label,
        Button,
        Button,
        Button,
        Label,
        Button,
    ) {
        let root = Box::new(Orientation::Vertical, 6);
        root.set_margin_top(6);
        root.set_margin_bottom(6);
        root.set_margin_start(6);
        root.set_margin_end(6);

        // Header with battery on left and power icon on right
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_valign(Align::Center);
        let battery_label = Label::builder()
            .label("󰁹 98%")
            .css_classes(vec!["qs-primary"])
            .xalign(0.0)
            .hexpand(true)
            .build();
        let power_btn = Button::builder()
            .label("󰐥")
            .build();
        power_btn.set_css_classes(&["header-power-btn"]);
        header.append(&battery_label);
        header.append(&power_btn);
        root.append(&header);

        let grid = Grid::builder()
            .column_spacing(6)
            .row_spacing(6)
            .halign(Align::Fill)
            .valign(Align::Fill)
            .build();
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(false);
        grid.set_margin_top(6);

        let (wifi_tile, wifi_label, wifi_sublabel) = tiles::tile_with_labels(
            "network-wireless-signal-excellent-symbolic",
            "Wi-Fi",
            "Loading...",
        );
        wifi_tile.set_css_classes(&["qs-tile"]);
        
        let (bt_tile, bt_label, bt_sublabel) =
            tiles::tile_with_labels("bluetooth-active-symbolic", "Bluetooth", "Loading...");
        bt_tile.set_css_classes(&["qs-tile"]);

        let (warp_tile, warp_label, warp_sublabel) =
            tiles::tile_with_labels("network-vpn-symbolic", "Warp", "Loading...");
        warp_tile.set_css_classes(&["qs-tile"]);

        let dnd_tile = tiles::tile("notifications-disabled-symbolic", "DND", "Placeholder");
        dnd_tile.set_css_classes(&["qs-tile"]);

        let av_card = Self::build_av_card();
        let (power_profile_card, saver_btn, balanced_btn, perf_btn) = Self::build_power_profile_card();

        grid.attach(&wifi_tile, 0, 0, 1, 1);
        grid.attach(&bt_tile, 1, 0, 1, 1);
        grid.attach(&warp_tile, 0, 1, 1, 1);
        grid.attach(&dnd_tile, 1, 1, 1, 1);
        grid.attach(&av_card.0, 0, 2, 2, 1);
        grid.attach(&power_profile_card, 0, 3, 2, 1);

        root.append(&grid);

        (
            root,
            av_card.1,
            av_card.2,
            av_card.3,
            av_card.4,
            av_card.5,
            av_card.6,
            wifi_tile,
            bt_tile,
            warp_tile,
            wifi_label,
            wifi_sublabel,
            bt_label,
            bt_sublabel,
            warp_label,
            warp_sublabel,
            saver_btn,
            balanced_btn,
            perf_btn,
            battery_label,
            power_btn,
        )
    }

    fn build_av_card() -> (Box, Scale, Scale, Label, Button, Revealer, ListBox) {
        let card = Box::new(Orientation::Vertical, 8);
        card.set_css_classes(&["qs-card"]);
        card.set_margin_top(1);
        card.set_margin_bottom(1);
        card.set_margin_start(1);
        card.set_margin_end(1);

        // Brightness row
        let brightness_row = Box::new(Orientation::Horizontal, 8);
        brightness_row.set_valign(Align::Center);
        let brightness_icon = Label::builder()
            .label("󰃠")
            .css_classes(vec!["qs-icon"])
            .xalign(0.0)
            .build();
        let brightness_slider = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        brightness_slider.set_value(50.0);
        brightness_slider.set_draw_value(false);
        brightness_slider.set_hexpand(true);
        brightness_row.append(&brightness_icon);
        brightness_row.append(&brightness_slider);
        card.append(&brightness_row);

        // Volume row with chevron
        let volume_row = Box::new(Orientation::Horizontal, 8);
        volume_row.set_valign(Align::Center);
        let volume_icon = Label::builder()
            .label("")
            .css_classes(vec!["qs-icon"])
            .xalign(0.0)
            .build();
        let volume_slider = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        volume_slider.set_value(65.0);
        volume_slider.set_draw_value(false);
        volume_slider.set_hexpand(true);
        let volume_label = Label::builder()
            .label("65%")
            .xalign(1.0)
            .css_classes(vec!["qs-secondary"])
            .build();
        let toggle_btn = Button::with_label(">");
        volume_row.append(&volume_icon);
        volume_row.append(&volume_slider);
        volume_row.append(&volume_label);
        volume_row.append(&toggle_btn);
        card.append(&volume_row);

        // Outputs list
        let outputs_list = ListBox::new();
        outputs_list.set_selection_mode(gtk4::SelectionMode::Single);
        outputs_list.set_css_classes(&["qs-list"]);

        let outputs_box = Box::new(Orientation::Vertical, 4);
        outputs_box.append(&outputs_list);

        let revealer = Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(100);
        revealer.set_reveal_child(false);
        revealer.set_child(Some(&outputs_box));
        card.append(&revealer);

        (card, brightness_slider, volume_slider, volume_label, toggle_btn, revealer, outputs_list)
    }

    fn build_power_profile_card() -> (Box, Button, Button, Button) {
        let card = Box::new(Orientation::Vertical, 6);
        card.set_css_classes(&["qs-card"]);
        card.set_margin_top(1);
        card.set_margin_bottom(1);
        card.set_margin_start(1);
        card.set_margin_end(1);

        // Header row with title only (power menu moved to top header)
        let header = Label::builder()
            .label("Power Profile")
            .xalign(0.0)
            .css_classes(vec!["qs-primary"])
            .build();
        card.append(&header);

        // Profile buttons row
        let button_box = Box::new(Orientation::Horizontal, 4);
        button_box.set_homogeneous(true);

        let saver_btn = Button::with_label("Saver");
        saver_btn.set_css_classes(&["power-profile-btn"]);
        
        let balanced_btn = Button::with_label("Balanced");
        balanced_btn.set_css_classes(&["power-profile-btn", "active"]);
        
        let perf_btn = Button::with_label("Perf");
        perf_btn.set_css_classes(&["power-profile-btn"]);

        button_box.append(&saver_btn);
        button_box.append(&balanced_btn);
        button_box.append(&perf_btn);

        card.append(&button_box);

        // Set current profile on load
        {
            let saver_btn_clone = saver_btn.clone();
            let balanced_btn_clone = balanced_btn.clone();
            let perf_btn_clone = perf_btn.clone();
            
            let handle = runtime::handle();
            glib::spawn_future_local(async move {
                let profile_result = handle.spawn(async move {
                    if let Ok(output) = tokio::process::Command::new("powerprofilesctl")
                        .arg("get")
                        .output()
                        .await
                    {
                        if let Ok(profile) = String::from_utf8(output.stdout) {
                            return Some(profile.trim().to_string());
                        }
                    }
                    None
                }).await;
                
                if let Ok(Some(profile)) = profile_result {
                    saver_btn_clone.remove_css_class("active");
                    balanced_btn_clone.remove_css_class("active");
                    perf_btn_clone.remove_css_class("active");
                    
                    match profile.as_str() {
                        "power-saver" => saver_btn_clone.add_css_class("active"),
                        "balanced" => balanced_btn_clone.add_css_class("active"),
                        "performance" => perf_btn_clone.add_css_class("active"),
                        _ => {}
                    }
                }
            });
        }

        // Handle button clicks
        {
            let saver_btn_clone = saver_btn.clone();
            let balanced_btn_clone = balanced_btn.clone();
            let perf_btn_clone = perf_btn.clone();
            
            saver_btn.connect_clicked(move |_| {
                saver_btn_clone.add_css_class("active");
                balanced_btn_clone.remove_css_class("active");
                perf_btn_clone.remove_css_class("active");
                
                let handle = runtime::handle();
                handle.spawn(async move {
                    let _ = tokio::process::Command::new("powerprofilesctl")
                        .arg("set")
                        .arg("power-saver")
                        .output()
                        .await;
                });
            });
        }

        {
            let saver_btn_clone = saver_btn.clone();
            let balanced_btn_clone = balanced_btn.clone();
            let perf_btn_clone = perf_btn.clone();
            
            balanced_btn.connect_clicked(move |_| {
                saver_btn_clone.remove_css_class("active");
                balanced_btn_clone.add_css_class("active");
                perf_btn_clone.remove_css_class("active");
                
                let handle = runtime::handle();
                handle.spawn(async move {
                    let _ = tokio::process::Command::new("powerprofilesctl")
                        .arg("set")
                        .arg("balanced")
                        .output()
                        .await;
                });
            });
        }

        {
            let saver_btn_clone = saver_btn.clone();
            let balanced_btn_clone = balanced_btn.clone();
            let perf_btn_clone = perf_btn.clone();
            
            perf_btn.connect_clicked(move |_| {
                saver_btn_clone.remove_css_class("active");
                balanced_btn_clone.remove_css_class("active");
                perf_btn_clone.add_css_class("active");
                
                let handle = runtime::handle();
                handle.spawn(async move {
                    let _ = tokio::process::Command::new("powerprofilesctl")
                        .arg("set")
                        .arg("performance")
                        .output()
                        .await;
                });
            });
        }

        (card, saver_btn, balanced_btn, perf_btn)
    }

    // volume list functionality reused within AV card

    async fn refresh_outputs_ui(&self) {
        let handle = runtime::handle();
        let result = handle
            .spawn({
                let client = Arc::clone(&self.device_client);
                async move { client.audio_outputs().await }
            })
            .await;

        let (outputs, default_id) = match result {
            Ok(Ok(list)) => {
                let default_id = list.iter().find(|o| o.is_default).map(|o| o.id.clone());
                (list, default_id)
            }
            Ok(Err(e)) => {
                eprintln!("Audio outputs error: {e}");
                (Vec::new(), None)
            }
            Err(e) => {
                eprintln!("Audio outputs task error: {e}");
                (Vec::new(), None)
            }
        };

        {
            let mut cache = self.outputs.borrow_mut();
            *cache = outputs.clone();
        }

        Self::rebuild_outputs_list(&self.outputs_list, &outputs, default_id.as_deref());
    }

    fn rebuild_outputs_list(list: &ListBox, outputs: &[AudioOutput], default_id: Option<&str>) {
        let mut current_row = list.first_child();

        // Update existing rows or add new ones
        for output in outputs {
            let row = if let Some(existing) = current_row.as_ref() {
                let r = existing.clone().downcast::<ListBoxRow>().unwrap();
                current_row = existing.next_sibling();
                r
            } else {
                let r = ListBoxRow::new();
                r.set_selectable(true);
                r.set_activatable(true);
                list.append(&r);
                r
            };

            // Update or create child content
            if let Some(child) = row.child() {
                if let Ok(row_box) = child.downcast::<Box>() {
                    // Update existing label
                    if let Some(label) = row_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                        label.set_label(&output.name);
                    }
                    // Update check mark visibility
                    let should_show = output.is_default;
                    if let Some(check) = row_box.last_child() {
                        if row_box.first_child() != row_box.last_child() {
                            check.set_visible(should_show);
                        } else if should_show {
                            let check_label = Label::builder()
                                .label("*")
                                .xalign(1.0)
                                .css_classes(vec!["qs-primary"])
                                .build();
                            row_box.append(&check_label);
                        }
                    } else if should_show {
                        let check_label = Label::builder()
                            .label("*")
                            .xalign(1.0)
                            .css_classes(vec!["qs-primary"])
                            .build();
                        row_box.append(&check_label);
                    }
                    continue;
                }
            }

            // Create new content
            let row_box = Box::new(Orientation::Horizontal, 8);
            row_box.set_margin_top(6);
            row_box.set_margin_bottom(6);
            row_box.set_margin_start(6);
            row_box.set_margin_end(6);

            let name = Label::builder()
                .label(&output.name)
                .xalign(0.0)
                .css_classes(vec!["qs-primary"])
                .build();
            name.set_hexpand(true);
            row_box.append(&name);

            if output.is_default {
                let check = Label::builder()
                    .label("*")
                    .xalign(1.0)
                    .css_classes(vec!["qs-primary"])
                    .build();
                row_box.append(&check);
            }

            row.set_child(Some(&row_box));
        }

        // Remove excess rows
        while let Some(child) = current_row {
            let next = child.next_sibling();
            list.remove(&child);
            current_row = next;
        }

        // Handle empty list
        if outputs.is_empty() && list.first_child().is_none() {
            let row = ListBoxRow::new();
            let label = Label::builder()
                .label("No outputs found")
                .xalign(0.0)
                .css_classes(vec!["qs-secondary"])
                .build();
            row.set_child(Some(&label));
            list.append(&row);
            list.unselect_all();
        }

        // Set default selection
        if let Some(id) = default_id {
            if let Some(idx) = outputs.iter().position(|o| o.id == id) {
                if let Some(row) = list.row_at_index(idx as i32) {
                    list.select_row(Some(&row));
                }
            }
        }
    }

    fn apply_styles(window: &ApplicationWindow) {
        let css = gtk4::CssProvider::new();
        css.load_from_data(
            "
            window {
                background-color: rgba(30,30,30,0.9);
                color: #f5f5f5;
                font-size: 10pt;
                border-radius: 16px;
                border: 1px solid rgba(255,255,255,0.06);
            }
            
            button {
                border: none;
                border-radius: 8px;
                padding: 6px 12px;
                font-size: 9pt;
            }
            
            button.qs-tile {
                background: rgba(255,255,255,0.06);
                border-radius: 14px;
                padding: 0;
            }
            button.qs-tile:hover {
                background: rgba(255,255,255,0.10);
            }
            
            .qs-card {
                background: rgba(255,255,255,0.04);
                border-radius: 14px;
                border: 1px solid rgba(255,255,255,0.06);
                padding: 10px;
            }
            
            .qs-primary {
                font-weight: 600;
                color: #f5f5f5;
            }
            
            .qs-secondary {
                color: rgba(245,245,245,0.65);
                font-size: 9pt;
            }
            .qs-secondary.connected {
                color: rgb(58,200,134);
            }
            
            .qs-icon {
                color: rgba(245,245,245,0.85);
                font-size: 11pt;
                min-width: 16px;
            }
            
            .qs-list row {
                padding: 8px 6px;
            }
            .qs-list row:selected {
                background-color: rgba(255,255,255,0.15);
            }
            
            scale {
                min-height: 24px;
                margin: 3px 0;
            }
            scale trough {
                background: rgba(255,255,255,0.10);
                border-radius: 5px;
                min-height: 5px;
                margin: 6px 0;
            }
            scale slider {
                background: rgba(255,255,255,0.95);
                border: 1px solid rgba(0,0,0,0.15);
                border-radius: 50%;
                min-width: 18px;
                min-height: 18px;
                margin: -6px 0;
                box-shadow: 0 2px 4px rgba(0,0,0,0.2);
            }
            scale slider:hover {
                background: rgba(255,255,255,0.99);
                box-shadow: 0 4px 8px rgba(0,0,0,0.25);
            }
            
            .power-profile-btn {
                background: rgba(255,255,255,0.08);
                padding: 6px 4px;
                color: rgba(245,245,245,0.65);
                font-weight: 500;
            }
            .power-profile-btn:hover {
                background: rgba(255,255,255,0.12);
            }
            .power-profile-btn.active {
                background: rgba(58,134,255,0.90);
                color: #f5f5f5;
                border: 2px solid rgba(58,134,255,0.95);
                padding: 4px 2px;
            }
            
            .header-power-btn {
                background: rgba(255,255,255,0.08);
                min-width: 32px;
                min-height: 32px;
                font-size: 12pt;
                color: rgba(245,245,245,0.85);
                padding: 0;
                margin: 0;
            }
            .header-power-btn:hover {
                background: rgba(255,255,255,0.12);
            }
            
            .power-menu-item {
                background: transparent;
                padding: 8px 16px;
                color: #f5f5f5;
            }
            .power-menu-item:hover {
                background: rgba(255,255,255,0.10);
            }
            
            popover {
                background: rgba(35,35,35,0.98);
                border: 1px solid rgba(255,255,255,0.10);
                border-radius: 10px;
                padding: 4px;
            }
            "
        );

        gtk4::style_context_add_provider_for_display(
            &gtk4::prelude::RootExt::display(window),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    pub fn show(&self) {
        self.window.present();
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub async fn toggle(&self) {
        let mut visible = self.is_visible.write().await;
        *visible = !*visible;

        if *visible {
            self.show();
            let qs = Arc::new(self.clone_for_update());
            glib::spawn_future_local(async move {
                qs.update_wifi_status().await;
                qs.update_bluetooth_status().await;
            });
        } else {
            self.hide();
        }
    }

    async fn update_wifi_status(&self) {
        let handle = runtime::handle();
        let result = handle
            .spawn({
                let client = Arc::clone(&self.device_client);
                async move { client.wifi_connected().await }
            })
            .await;

        let new_status = match result {
            Ok(Ok(Some(ref network))) => format!("connected:{}", network.ssid),
            Ok(Ok(None)) => "disconnected".to_string(),
            _ => "error".to_string(),
        };

        // Only update UI if status changed
        let cached = self.cached_wifi_status.borrow().clone();
        if cached == new_status {
            return;
        }
        *self.cached_wifi_status.borrow_mut() = new_status.clone();

        match result {
            Ok(Ok(Some(network))) => {
                self.wifi_tile_label.set_label(&network.ssid);
                self.wifi_tile_sublabel.set_label("● Connected");
                self.wifi_tile_sublabel.set_css_classes(&["qs-secondary", "connected"]);
            }
            Ok(Ok(None)) => {
                self.wifi_tile_label.set_label("Wi-Fi");
                self.wifi_tile_sublabel.set_label("Not connected");
                self.wifi_tile_sublabel.set_css_classes(&["qs-secondary"]);
            }
            Ok(Err(e)) => {
                eprintln!("WiFi status error: {}", e);
                self.wifi_tile_label.set_label("Wi-Fi");
                self.wifi_tile_sublabel.set_label("Unavailable");
                self.wifi_tile_sublabel.set_css_classes(&["qs-secondary"]);
            }
            Err(e) => {
                eprintln!("WiFi status task error: {}", e);
                self.wifi_tile_label.set_label("Wi-Fi");
                self.wifi_tile_sublabel.set_label("Unavailable");
                self.wifi_tile_sublabel.set_css_classes(&["qs-secondary"]);
            }
        }
    }

    async fn update_bluetooth_status(&self) {
        let handle = runtime::handle();
        let result = handle
            .spawn({
                let client = Arc::clone(&self.device_client);
                async move { client.bluetooth_connected().await }
            })
            .await;

        let new_status = match result {
            Ok(Ok(Some(ref device))) => format!("connected:{}", device.name),
            Ok(Ok(None)) => "disconnected".to_string(),
            _ => "error".to_string(),
        };

        // Check if BT status actually changed for audio refresh
        let cached = self.cached_bt_status.borrow().clone();
        let bt_changed = !cached.is_empty() && cached != new_status 
            && (cached.starts_with("connected:") != new_status.starts_with("connected:"));
        
        // Only update UI if status changed
        if cached == new_status {
            return;
        }
        *self.cached_bt_status.borrow_mut() = new_status.clone();

        match result {
            Ok(Ok(Some(device))) => {
                self.bt_tile_label.set_label(&device.name);
                self.bt_tile_sublabel.set_label("● Connected");
                self.bt_tile_sublabel.set_css_classes(&["qs-secondary", "connected"]);
            }
            Ok(Ok(None)) => {
                self.bt_tile_label.set_label("Bluetooth");
                self.bt_tile_sublabel.set_label("Not connected");
                self.bt_tile_sublabel.set_css_classes(&["qs-secondary"]);
            }
            Ok(Err(e)) => {
                eprintln!("Bluetooth status error: {}", e);
                self.bt_tile_label.set_label("Bluetooth");
                self.bt_tile_sublabel.set_label("Unavailable");
                self.bt_tile_sublabel.set_css_classes(&["qs-secondary"]);
            }
            Err(e) => {
                eprintln!("Bluetooth status task error: {}", e);
                self.bt_tile_label.set_label("Bluetooth");
                self.bt_tile_sublabel.set_label("Unavailable");
                self.bt_tile_sublabel.set_css_classes(&["qs-secondary"]);
            }
        }

        // Refresh audio devices instantly when bluetooth status changes
        if bt_changed {
            eprintln!("[AUDIO] Bluetooth status changed, refreshing audio devices...");
            // Small delay to let the audio system register the new device
            let qs = Arc::new(self.clone_for_update());
            glib::timeout_add_local(
                std::time::Duration::from_millis(500),
                move || {
                    let qs = Arc::clone(&qs);
                    glib::spawn_future_local(async move {
                        qs.refresh_outputs_ui().await;
                    });
                    glib::ControlFlow::Break
                },
            );
        }
    }

    async fn update_warp_status(&self) {
        let handle = runtime::handle();
        let result = handle
            .spawn(async move {
                if let Ok(output) = tokio::process::Command::new("warp-cli")
                    .arg("status")
                    .output()
                    .await
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok::<bool, std::io::Error>(stdout.contains("Connected"))
                } else {
                    Ok(false)
                }
            })
            .await;

        let new_status = match result {
            Ok(Ok(true)) => "connected".to_string(),
            Ok(Ok(false)) => "disconnected".to_string(),
            _ => "error".to_string(),
        };

        // Only update UI if status changed
        let cached = self.cached_warp_status.borrow().clone();
        if cached == new_status {
            return;
        }
        *self.cached_warp_status.borrow_mut() = new_status.clone();

        match result {
            Ok(Ok(is_connected)) => {
                if is_connected {
                    self.warp_tile_label.set_label("Warp");
                    self.warp_tile_sublabel.set_label("● Connected");
                    self.warp_tile_sublabel.set_css_classes(&["qs-secondary", "connected"]);
                } else {
                    self.warp_tile_label.set_label("Warp");
                    self.warp_tile_sublabel.set_label("Disconnected");
                    self.warp_tile_sublabel.set_css_classes(&["qs-secondary"]);
                }
            }
            _ => {
                self.warp_tile_label.set_label("Warp");
                self.warp_tile_sublabel.set_label("Unavailable");
                self.warp_tile_sublabel.set_css_classes(&["qs-secondary"]);
            }
        }
    }

    async fn update_battery_status(&self) {
        let handle = runtime::handle();
        let percentage = handle
            .spawn(async move {
                let Ok(output) = tokio::process::Command::new("upower")
                    .args(["-e"])
                    .output()
                    .await else { return None; };
                
                let stdout = String::from_utf8_lossy(&output.stdout);
                let bat_line = stdout.lines().find(|l| l.contains("BAT"))?;
                
                let Ok(info) = tokio::process::Command::new("upower")
                    .arg("-i")
                    .arg(bat_line.trim())
                    .output()
                    .await else { return None; };
                
                let info_str = String::from_utf8_lossy(&info.stdout);
                info_str.lines()
                    .find(|l| l.contains("percentage:"))?
                    .split(':')
                    .nth(1)?
                    .trim()
                    .trim_end_matches('%')
                    .parse::<u32>()
                    .ok()
            })
            .await
            .ok()
            .flatten();

        if let Some(pct) = percentage {
            let icon = match pct {
                80..=100 => "󰁹",
                50..=79 => "󰂀",
                20..=49 => "󰂂",
                _ => "󰂃",
            };
            self.battery_label.set_label(&format!("{} {}%", icon, pct));
        }
    }

    async fn toggle_warp(&self) {
        let handle = runtime::handle();
        let is_connected = {
            let label = self.warp_tile_sublabel.label();
            label.contains("Connected")
        };

        let cmd = if is_connected { "disconnect" } else { "connect" };
        eprintln!("[WARP] Running: warp-cli {}", cmd);

        let qs = Arc::new(self.clone_for_update());
        let result = handle
            .spawn(async move {
                tokio::process::Command::new("warp-cli")
                    .arg(cmd)
                    .output()
                    .await
            })
            .await;

        match result {
            Ok(Ok(_)) => {
                // Update immediately
                glib::timeout_add_local(
                    std::time::Duration::from_millis(500),
                    {
                        let qs = Arc::clone(&qs);
                        move || {
                            let qs = Arc::clone(&qs);
                            glib::spawn_future_local(async move {
                                qs.update_warp_status().await;
                            });
                            glib::ControlFlow::Break
                        }
                    },
                );
            }
            _ => {
                eprintln!("[WARP] Toggle failed");
                qs.update_warp_status().await;
            }
        }
    }
}

#[allow(dead_code)]
fn read_battery() -> anyhow::Result<u32> {
    let capacity = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity")?
        .trim()
        .parse::<u32>()?;
    Ok(capacity)
}

// Set volume for default sink via wpctl
async fn set_volume_direct(percentage: u32) -> anyhow::Result<()> {
    // Convert percentage (0-100) to volume (0.0-1.0)
    let volume = (percentage as f64) / 100.0;
    
    // Set volume for default sink
    tokio::process::Command::new("wpctl")
        .arg("set-volume")
        .arg("@DEFAULT_AUDIO_SINK@")
        .arg(format!("{:.2}", volume))
        .output()
        .await?;
    
    Ok(())
}

// Get volume of specific audio device via wpctl
async fn get_device_volume(sink_id: &str) -> anyhow::Result<u32> {
    let output = tokio::process::Command::new("wpctl")
        .arg("get-volume")
        .arg(sink_id)
        .output()
        .await?;

    let vol_str = String::from_utf8(output.stdout)?;
    
    // Parse "Volume: 0.65" or "Volume: 0.65 [MUTED]" format
    if let Some(vol_part) = vol_str.split("Volume: ").nth(1) {
        let trimmed = vol_part.trim();
        // Handle "[MUTED]" suffix
        let vol_only = trimmed.split_whitespace().next().unwrap_or(trimmed);
        if let Ok(vol_f) = vol_only.parse::<f64>() {
            let percentage = ((vol_f * 100.0) as u32).min(100);
            return Ok(percentage);
        }
    }
    
    Ok(65) // fallback
}

// Get volume of default audio sink as fallback
async fn get_default_volume() -> anyhow::Result<u32> {
    let output = tokio::process::Command::new("wpctl")
        .arg("get-volume")
        .arg("@DEFAULT_AUDIO_SINK@")
        .output()
        .await?;

    let vol_str = String::from_utf8(output.stdout)?;
    
    if let Some(vol_part) = vol_str.split("Volume: ").nth(1) {
        let trimmed = vol_part.trim();
        let vol_only = trimmed.split_whitespace().next().unwrap_or(trimmed);
        if let Ok(vol_f) = vol_only.parse::<f64>() {
            let percentage = ((vol_f * 100.0) as u32).min(100);
            return Ok(percentage);
        }
    }
    
    Ok(65)
}

