use crate::services::device_client::DeviceClient;
use crate::services::network::WifiNetwork;
use crate::services::runtime;
use gtk4::prelude::*;
use gtk4::{glib, Box, Button, Image, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub struct WifiView {
    pub root: Box,
    pub back_btn: Button,
    #[allow(dead_code)]
    list: ListBox,
    #[allow(dead_code)]
    device_client: Arc<DeviceClient>,
    #[allow(dead_code)]
    selection: Rc<RefCell<Option<String>>>,
    #[allow(dead_code)]
    networks: Rc<RefCell<Vec<WifiNetwork>>>,
    #[allow(dead_code)]
    status_label: Label,
    #[allow(dead_code)]
    connect_btn: Button,
    #[allow(dead_code)]
    disconnect_btn: Button,
}

impl WifiView {
    pub fn new(device_client: Arc<DeviceClient>) -> Self {
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
        list.set_selection_mode(gtk4::SelectionMode::Single);
        scrolled.set_child(Some(&list));
        root.append(&scrolled);

        let footer = Box::new(Orientation::Horizontal, 8);
        footer.set_margin_top(8);
        footer.set_margin_bottom(4);
        footer.set_margin_start(4);
        footer.set_margin_end(4);
        footer.set_halign(gtk4::Align::Fill);

        let status_label = Label::builder()
            .label("Select a network")
            .xalign(0.0)
            .css_classes(vec!["qs-secondary"])
            .wrap(true)
            .build();
        status_label.set_hexpand(true);

        let connect_btn = Button::with_label("Connect");
        connect_btn.set_sensitive(false);
        connect_btn.set_css_classes(&["suggested-action"]);

        let disconnect_btn = Button::with_label("Disconnect");
        disconnect_btn.set_sensitive(false);
        disconnect_btn.set_css_classes(&["destructive-action"]);

        footer.append(&status_label);
        footer.append(&connect_btn);
        footer.append(&disconnect_btn);
        root.append(&footer);

        let selection = Rc::new(RefCell::new(None));
        let networks = Rc::new(RefCell::new(Vec::new()));

        let wifi_view = Self {
            root: root.clone(),
            back_btn,
            list: list.clone(),
            device_client: Arc::clone(&device_client),
            selection: selection.clone(),
            networks: networks.clone(),
            status_label: status_label.clone(),
            connect_btn: connect_btn.clone(),
            disconnect_btn: disconnect_btn.clone(),
        };

        // Initial load when mapped
        {
            let device = Arc::clone(&device_client);
            let list_ref = list.clone();
            let selection_ref = selection.clone();
            let networks_ref = networks.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            root.connect_map(move |_| {
                let device = Arc::clone(&device);
                let list = list_ref.clone();
                let selection = selection_ref.clone();
                let networks = networks_ref.clone();
                let status = status_ref.clone();
                let connect = connect_ref.clone();
                let disconnect = disconnect_ref.clone();
                glib::spawn_future_local(async move {
                    refresh_networks_impl(&list, &device, &selection, &networks, &status, &connect, &disconnect).await;
                });
            });
        }

        // Periodic refresh while visible
        {
            let device = Arc::clone(&device_client);
            let list_ref = list.clone();
            let selection_ref = selection.clone();
            let networks_ref = networks.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            let root_weak = root.downgrade();
            glib::timeout_add_seconds_local(5, move || {
                if let Some(root) = root_weak.upgrade() {
                    if root.is_visible() {
                        let device = Arc::clone(&device);
                        let list = list_ref.clone();
                        let selection = selection_ref.clone();
                        let networks = networks_ref.clone();
                        let status = status_ref.clone();
                        let connect = connect_ref.clone();
                        let disconnect = disconnect_ref.clone();
                        glib::spawn_future_local(async move {
                            refresh_networks_impl(
                                &list,
                                &device,
                                &selection,
                                &networks,
                                &status,
                                &connect,
                                &disconnect,
                            )
                            .await;
                        });
                    }
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Continue
                }
            });
        }

        // Selection change drives footer actions
        {
            let selection_ref = selection.clone();
            let networks_ref = networks.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            list.connect_selected_rows_changed(move |lb| {
                let selected = lb.selected_row().and_then(|row| {
                    let idx = row.index();
                    if idx >= 0 {
                        networks_ref
                            .borrow()
                            .get(idx as usize)
                            .map(|n| n.ssid.clone())
                    } else {
                        None
                    }
                });
                *selection_ref.borrow_mut() = selected.clone();
                update_footer(
                    &selection_ref,
                    &status_ref,
                    &connect_ref,
                    &disconnect_ref,
                    lb,
                    &networks_ref,
                );
            });
        }

        // Footer buttons
        {
            let selection_ref = selection.clone();
            let networks_ref = networks.clone();
            let client = Arc::clone(&device_client);
            let list_ref = list.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            connect_btn.connect_clicked(move |btn| {
                if let Some(ssid) = selection_ref.borrow().clone() {
                    btn.set_label("Connecting...");
                    btn.set_sensitive(false);
                    let client = Arc::clone(&client);
                    let list = list_ref.clone();
                    let selection = selection_ref.clone();
                    let networks = networks_ref.clone();
                    let status = status_ref.clone();
                    let connect = connect_ref.clone();
                    let disconnect = disconnect_ref.clone();
                    glib::spawn_future_local(async move {
                        let handle = runtime::handle();
                        let _ = handle
                            .spawn({
                                let client = Arc::clone(&client);
                                let ssid = ssid.clone();
                                async move { client.wifi_connect(&ssid).await }
                            })
                            .await;
                        refresh_networks_impl(&list, &client, &selection, &networks, &status, &connect, &disconnect)
                            .await;
                    });
                }
            });
        }

        {
            let selection_ref = selection.clone();
            let networks_ref = networks.clone();
            let client = Arc::clone(&device_client);
            let list_ref = list.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            disconnect_btn.connect_clicked(move |btn| {
                btn.set_label("Disconnecting...");
                btn.set_sensitive(false);
                let client = Arc::clone(&client);
                let list = list_ref.clone();
                let selection = selection_ref.clone();
                let networks = networks_ref.clone();
                let status = status_ref.clone();
                let connect = connect_ref.clone();
                let disconnect = disconnect_ref.clone();
                glib::spawn_future_local(async move {
                    let handle = runtime::handle();
                    let _ = handle
                        .spawn({
                            let client = Arc::clone(&client);
                            async move { client.wifi_disconnect().await }
                        })
                        .await;
                    refresh_networks_impl(&list, &client, &selection, &networks, &status, &connect, &disconnect)
                        .await;
                });
            });
        }

        wifi_view
    }
}

async fn refresh_networks_impl(
    list: &ListBox,
    device_client: &Arc<DeviceClient>,
    selection: &Rc<RefCell<Option<String>>>,
    networks: &Rc<RefCell<Vec<WifiNetwork>>>,
    status_label: &Label,
    connect_btn: &Button,
    disconnect_btn: &Button,
) {
    let handle = runtime::handle();
    let result = handle
        .spawn({
            let client = Arc::clone(device_client);
            async move { client.wifi_list().await }
        })
        .await;

    match result {
        Ok(Ok(mut nets)) => {
            let previous = selection.borrow().clone();
            nets.sort_by(|a, b| b.strength.cmp(&a.strength));
            *networks.borrow_mut() = nets.clone();

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            if nets.is_empty() {
                let row = ListBoxRow::new();
                let label = Label::builder()
                    .label("No networks found")
                    .xalign(0.0)
                    .css_classes(vec!["qs-secondary"])
                    .build();
                row.set_child(Some(&label));
                list.append(&row);
            } else {
                for network in &nets {
                    list.append(&create_network_row(network));
                }
            }

            if let Some(prev_ssid) = previous {
                if let Some(idx) = nets.iter().position(|n| n.ssid == prev_ssid) {
                    if let Some(row) = list.row_at_index(idx as i32) {
                        list.select_row(Some(&row));
                    }
                } else {
                    list.unselect_all();
                }
            } else {
                list.unselect_all();
            }

            update_footer(selection, status_label, connect_btn, disconnect_btn, list, networks);
        }
        Ok(Err(err)) => {
            eprintln!("Failed to list wifi networks: {err}");
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let row = ListBoxRow::new();
            let label = Label::builder()
                .label(&format!("Error: {err}"))
                .xalign(0.0)
                .css_classes(vec!["qs-secondary"])
                .build();
            row.set_child(Some(&label));
            list.append(&row);
            *selection.borrow_mut() = None;
            networks.borrow_mut().clear();
            update_footer(selection, status_label, connect_btn, disconnect_btn, list, networks);
        }
        Err(err) => {
            eprintln!("Failed to join wifi task: {err}");
        }
    }
}

fn create_network_row(network: &WifiNetwork) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_selectable(true);
    row.set_activatable(true);

    let row_box = Box::new(Orientation::Horizontal, 12);
    row_box.set_margin_top(8);
    row_box.set_margin_bottom(8);
    row_box.set_margin_start(12);
    row_box.set_margin_end(12);

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

    let right_box = Box::new(Orientation::Horizontal, 8);
    right_box.set_halign(gtk4::Align::End);
    right_box.set_hexpand(true);

    if network.secure {
        let lock_icon = Image::from_icon_name("network-wireless-encrypted-symbolic");
        lock_icon.set_pixel_size(14);
        right_box.append(&lock_icon);
    }

    row_box.append(&right_box);
    row.set_child(Some(&row_box));
    row
}

fn update_footer(
    selection: &Rc<RefCell<Option<String>>>,
    status_label: &Label,
    connect_btn: &Button,
    disconnect_btn: &Button,
    _list: &ListBox,
    networks: &Rc<RefCell<Vec<WifiNetwork>>>,
) {
    connect_btn.set_label("Connect");
    disconnect_btn.set_label("Disconnect");

    if let Some(ssid) = selection.borrow().clone() {
        let connected = networks
            .borrow()
            .iter()
            .find(|n| n.ssid == ssid)
            .map(|n| n.connected);

        if let Some(connected) = connected {
            let label_text = if connected {
                format!("{ssid} connected")
            } else {
                ssid.clone()
            };
            status_label.set_label(&label_text);
            connect_btn.set_sensitive(!connected);
            disconnect_btn.set_sensitive(connected);
            return;
        }
    }

    status_label.set_label("Select a network");
    connect_btn.set_sensitive(false);
    disconnect_btn.set_sensitive(false);
}
