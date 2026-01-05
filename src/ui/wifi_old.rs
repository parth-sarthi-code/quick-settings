use gtk4::prelude::*;
use gtk4::{glib, Box, Button, Image, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::network::NetworkService;

/// Build Wi-Fi subview with back button and list container
pub struct WifiView {
    pub root: Box,
    pub back_btn: Button,
    list: ListBox,
    network_service: Arc<RwLock<Option<Arc<NetworkService>>>>,
}

impl WifiView {
    pub fn new(network_service: Arc<RwLock<Option<Arc<NetworkService>>>>) -> Self {
        let root = Box::new(Orientation::Vertical, 12);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let header = Box::new(Orientation::Horizontal, 8);
        let back_btn = Button::with_label("←");
        back_btn.set_css_classes(&["qs-back"]);
        let title = Label::builder()
            .label("Wi-Fi")
            .xalign(0.0)
            .css_classes(vec!["qs-title"])
            .build();
        header.append(&back_btn);
        header.append(&title);
        root.append(&header);

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_height(280)
            .build();

        let list = ListBox::new();
        list.set_css_classes(&["qs-list"]);
        scrolled.set_child(Some(&list));
        root.append(&scrolled);

        let root_clone = root.clone();
        let wifi_view = Self {
            root,
            back_btn,
            list: list.clone(),
            network_service: Arc::clone(&network_service),
        };

        // Load networks when the view is mapped
        let wifi_view_ref = wifi_view.clone_for_async();
        root_clone.connect_map(move |_| {
            let wifi_view = wifi_view_ref.clone();
            glib::spawn_future_local(async move {
                wifi_view.refresh_networks().await;
            });
        });

        wifi_view
    }

    fn clone_for_async(&self) -> Arc<WifiViewAsync> {
        Arc::new(WifiViewAsync {
            list: self.list.clone(),
            network_service: Arc::clone(&self.network_service),
        })
    }

    async fn refresh_networks(&self) {
        // Clear current list
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        // Add loading indicator
        let loading_row = ListBoxRow::new();
        let loading_label = Label::builder()
            .label("Scanning for networks...")
            .xalign(0.0)
            .build();
        loading_row.set_child(Some(&loading_label));
        self.list.append(&loading_row);

        // Fetch networks
        match self.network_service.get_wifi_networks().await {
            Ok(networks) => {
                // Clear loading
                while let Some(child) = self.list.first_child() {
                    self.list.remove(&child);
                }

                if networks.is_empty() {
                    let empty_row = ListBoxRow::new();
                    let empty_label = Label::builder()
                        .label("No networks found")
                        .xalign(0.0)
                        .css_classes(vec!["qs-secondary"])
                        .build();
                    empty_row.set_child(Some(&empty_label));
                    self.list.append(&empty_row);
                } else {
                    for network in networks {
                        let row = self.create_network_row(&network);
                        self.list.append(&row);
                    }
                }
            }
            Err(e) => {
                // Clear loading
                while let Some(child) = self.list.first_child() {
                    self.list.remove(&child);
                }

                let error_row = ListBoxRow::new();
                let error_label = Label::builder()
                    .label(&format!("Error: {}", e))
                    .xalign(0.0)
                    .css_classes(vec!["qs-secondary"])
                    .build();
                error_row.set_child(Some(&error_label));
                self.list.append(&error_row);
            }
        }
    }

    fn create_network_row(&self, network: &crate::services::network::WifiNetwork) -> ListBoxRow {
        let row = ListBoxRow::new();
        let row_box = Box::new(Orientation::Horizontal, 12);
        row_box.set_margin_top(8);
        row_box.set_margin_bottom(8);
        row_box.set_margin_start(12);
        row_box.set_margin_end(12);

        // Signal strength icon
        let signal_icon = if network.strength >= 75 {
            "network-wireless-signal-excellent-symbolic"
        } else if network.strength >= 50 {
            "network-wireless-signal-good-symbolic"
        } else if network.strength >= 25 {
            "network-wireless-signal-ok-symbolic"
        } else {
            "network-wireless-signal-weak-symbolic"
        };
        let icon = Image::from_icon_name(signal_icon);
        icon.set_pixel_size(16);
        row_box.append(&icon);

        // Network name and status
        let text_box = Box::new(Orientation::Vertical, 2);
        let ssid_label = Label::builder()
            .label(&network.ssid)
            .xalign(0.0)
            .css_classes(vec!["qs-primary"])
            .build();
        text_box.append(&ssid_label);

        if network.connected {
            let status_label = Label::builder()
                .label("Connected")
                .xalign(0.0)
                .css_classes(vec!["qs-secondary"])
                .build();
            text_box.append(&status_label);
        }

        row_box.append(&text_box);

        // Lock icon for secure networks
        if network.secure {
            let lock_icon = Image::from_icon_name("network-wireless-encrypted-symbolic");
            lock_icon.set_pixel_size(14);
            lock_icon.set_halign(gtk4::Align::End);
            lock_icon.set_hexpand(true);
            row_box.append(&lock_icon);
        }

        row.set_child(Some(&row_box));

        // Connect on click (if not already connected)
        if !network.connected {
            let network_ssid = network.ssid.clone();
            let network_service = Arc::clone(&self.network_service);
            row.set_activatable(true);
            row.connect_activate(move |_| {
                let ssid = network_ssid.clone();
                let service = Arc::clone(&network_service);
                glib::spawn_future_local(async move {
                    match service.connect_network(&ssid).await {
                        Ok(_) => {
                            println!("Connected to {}", ssid);
                            // Refresh list after connection
                            // (In a full implementation, listen to NetworkManager signals)
                        }
                        Err(e) => {
                            eprintln!("Failed to connect to {}: {}", ssid, e);
                        }
                    }
                });
            });
        }

        row
    }
}

// Helper struct for async operations
struct WifiViewAsync {
    list: ListBox,
    network_service: Arc<NetworkService>,
}

impl WifiViewAsync {
    async fn refresh_networks(&self) {
        // Clear current list
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        // Add loading indicator
        let loading_row = ListBoxRow::new();
        let loading_label = Label::builder()
            .label("Scanning for networks...")
            .xalign(0.0)
            .build();
        loading_row.set_child(Some(&loading_label));
        self.list.append(&loading_row);

        // Fetch networks
        match self.network_service.get_wifi_networks().await {
            Ok(networks) => {
                // Clear loading
                while let Some(child) = self.list.first_child() {
                    self.list.remove(&child);
                }

                if networks.is_empty() {
                    let empty_row = ListBoxRow::new();
                    let empty_label = Label::builder()
                        .label("No networks found")
                        .xalign(0.0)
                        .css_classes(vec!["qs-secondary"])
                        .build();
                    empty_row.set_child(Some(&empty_label));
                    self.list.append(&empty_row);
                } else {
                    for network in networks {
                        let row = create_network_row(&network, &self.network_service, &self.list);
                        self.list.append(&row);
                    }
                }
            }
            Err(e) => {
                // Clear loading
                while let Some(child) = self.list.first_child() {
                    self.list.remove(&child);
                }

                let error_row = ListBoxRow::new();
                let error_label = Label::builder()
                    .label(&format!("Error: {}", e))
                    .xalign(0.0)
                    .css_classes(vec!["qs-secondary"])
                    .build();
                error_row.set_child(Some(&error_label));
                self.list.append(&error_row);
            }
        }
    }
}

fn create_network_row(
    network: &crate::services::network::WifiNetwork,
    network_service: &Arc<NetworkService>,
    _list: &ListBox,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    let row_box = Box::new(Orientation::Horizontal, 12);
    row_box.set_margin_top(8);
    row_box.set_margin_bottom(8);
    row_box.set_margin_start(12);
    row_box.set_margin_end(12);

    // Signal strength icon
    let signal_icon = if network.strength >= 75 {
        "network-wireless-signal-excellent-symbolic"
    } else if network.strength >= 50 {
        "network-wireless-signal-good-symbolic"
    } else if network.strength >= 25 {
        "network-wireless-signal-ok-symbolic"
    } else {
        "network-wireless-signal-weak-symbolic"
    };
    let icon = Image::from_icon_name(signal_icon);
    icon.set_pixel_size(16);
    row_box.append(&icon);

    // Network name and status
    let text_box = Box::new(Orientation::Vertical, 2);
    let ssid_label = Label::builder()
        .label(&network.ssid)
        .xalign(0.0)
        .css_classes(vec!["qs-primary"])
        .build();
    text_box.append(&ssid_label);

    if network.connected {
        let status_label = Label::builder()
            .label("Connected")
            .xalign(0.0)
            .css_classes(vec!["qs-secondary"])
            .build();
        text_box.append(&status_label);
    }

    row_box.append(&text_box);

    // Lock icon for secure networks
    if network.secure {
        let lock_icon = Image::from_icon_name("network-wireless-encrypted-symbolic");
        lock_icon.set_pixel_size(14);
        lock_icon.set_halign(gtk4::Align::End);
        lock_icon.set_hexpand(true);
        row_box.append(&lock_icon);
    }

    row.set_child(Some(&row_box));

    // Connect on click (if not already connected)
    if !network.connected {
        let network_ssid = network.ssid.clone();
        let network_service = Arc::clone(&network_service);
        row.set_activatable(true);
        row.connect_activate(move |_| {
            let ssid = network_ssid.clone();
            let service = Arc::clone(&network_service);
            glib::spawn_future_local(async move {
                match service.connect_network(&ssid).await {
                    Ok(_) => {
                        println!("Connected to {}", ssid);
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to {}: {}", ssid, e);
                    }
                }
            });
        });
    }

    row
}
