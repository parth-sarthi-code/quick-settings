use gtk4::prelude::*;
use gtk4::{glib, Box, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::services::device_client::DeviceClient;
use crate::services::deviced::BluetoothDevice;
use crate::services::runtime;

pub struct BluetoothView {
    pub root: Box,
    pub back_btn: Button,
    #[allow(dead_code)]
    list: ListBox,
    #[allow(dead_code)]
    device_client: Arc<DeviceClient>,
    #[allow(dead_code)]
    selection: Rc<RefCell<Option<String>>>,
    #[allow(dead_code)]
    devices: Rc<RefCell<Vec<BluetoothDevice>>>,
    #[allow(dead_code)]
    status_label: Label,
    #[allow(dead_code)]
    connect_btn: Button,
    #[allow(dead_code)]
    disconnect_btn: Button,
}

impl BluetoothView {
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
            .label("Bluetooth")
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
            .label("Select a device")
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
        let devices = Rc::new(RefCell::new(Vec::new()));

        let view = Self {
            root: root.clone(),
            back_btn,
            list: list.clone(),
            device_client: Arc::clone(&device_client),
            selection: selection.clone(),
            devices: devices.clone(),
            status_label: status_label.clone(),
            connect_btn: connect_btn.clone(),
            disconnect_btn: disconnect_btn.clone(),
        };

        // Initial refresh when mapped
        {
            let device = Arc::clone(&device_client);
            let list_ref = list.clone();
            let selection_ref = selection.clone();
            let devices_ref = devices.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            root.connect_map(move |_| {
                let device = Arc::clone(&device);
                let list = list_ref.clone();
                let selection = selection_ref.clone();
                let devices = devices_ref.clone();
                let status = status_ref.clone();
                let connect = connect_ref.clone();
                let disconnect = disconnect_ref.clone();
                glib::spawn_future_local(async move {
                    refresh_bluetooth_impl(
                        &list,
                        &device,
                        &selection,
                        &devices,
                        &status,
                        &connect,
                        &disconnect,
                    )
                    .await;
                });
            });
        }

        // Periodic refresh while visible
        {
            let device = Arc::clone(&device_client);
            let list_ref = list.clone();
            let selection_ref = selection.clone();
            let devices_ref = devices.clone();
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
                        let devices = devices_ref.clone();
                        let status = status_ref.clone();
                        let connect = connect_ref.clone();
                        let disconnect = disconnect_ref.clone();
                        glib::spawn_future_local(async move {
                            refresh_bluetooth_impl(
                                &list,
                                &device,
                                &selection,
                                &devices,
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

        // Selection drives footer state
        {
            let selection_ref = selection.clone();
            let devices_ref = devices.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            list.connect_selected_rows_changed(move |lb| {
                let selected = lb.selected_row().and_then(|row| {
                    let idx = row.index();
                    if idx >= 0 {
                        devices_ref
                            .borrow()
                            .get(idx as usize)
                            .map(|d| d.mac.clone())
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
                    &devices_ref,
                );
            });
        }

        // Footer buttons
        {
            let selection_ref = selection.clone();
            let devices_ref = devices.clone();
            let client = Arc::clone(&device_client);
            let list_ref = list.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            connect_btn.connect_clicked(move |btn| {
                if let Some(mac) = selection_ref.borrow().clone() {
                    btn.set_label("Connecting...");
                    btn.set_sensitive(false);
                    let client = Arc::clone(&client);
                    let list = list_ref.clone();
                    let selection = selection_ref.clone();
                    let devices = devices_ref.clone();
                    let status = status_ref.clone();
                    let connect = connect_ref.clone();
                    let disconnect = disconnect_ref.clone();
                    glib::spawn_future_local(async move {
                        let handle = runtime::handle();
                        let _ = handle
                            .spawn({
                                let client = Arc::clone(&client);
                                let mac = mac.clone();
                                async move { client.bluetooth_connect(&mac).await }
                            })
                            .await;
                        refresh_bluetooth_impl(
                            &list,
                            &client,
                            &selection,
                            &devices,
                            &status,
                            &connect,
                            &disconnect,
                        )
                        .await;
                    });
                }
            });
        }

        {
            let selection_ref = selection.clone();
            let devices_ref = devices.clone();
            let client = Arc::clone(&device_client);
            let list_ref = list.clone();
            let status_ref = status_label.clone();
            let connect_ref = connect_btn.clone();
            let disconnect_ref = disconnect_btn.clone();
            disconnect_btn.connect_clicked(move |btn| {
                if let Some(mac) = selection_ref.borrow().clone() {
                    btn.set_label("Disconnecting...");
                    btn.set_sensitive(false);
                    let client = Arc::clone(&client);
                    let list = list_ref.clone();
                    let selection = selection_ref.clone();
                    let devices = devices_ref.clone();
                    let status = status_ref.clone();
                    let connect = connect_ref.clone();
                    let disconnect = disconnect_ref.clone();
                    glib::spawn_future_local(async move {
                        let handle = runtime::handle();
                        let _ = handle
                            .spawn({
                                let client = Arc::clone(&client);
                                let mac = mac.clone();
                                async move { client.bluetooth_disconnect(&mac).await }
                            })
                            .await;
                        refresh_bluetooth_impl(
                            &list,
                            &client,
                            &selection,
                            &devices,
                            &status,
                            &connect,
                            &disconnect,
                        )
                        .await;
                    });
                }
            });
        }

        view
    }
}

async fn refresh_bluetooth_impl(
    list: &ListBox,
    device_client: &Arc<DeviceClient>,
    selection: &Rc<RefCell<Option<String>>>,
    devices: &Rc<RefCell<Vec<BluetoothDevice>>>,
    status_label: &Label,
    connect_btn: &Button,
    disconnect_btn: &Button,
) {
    let handle = runtime::handle();
    let result = handle
        .spawn({
            let client = Arc::clone(device_client);
            async move { client.bluetooth_list().await }
        })
        .await;

    match result {
        Ok(Ok(devs)) => {
            let previous = selection.borrow().clone();
            *devices.borrow_mut() = devs.clone();

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            if devs.is_empty() {
                let empty_row = ListBoxRow::new();
                let empty_label = Label::builder()
                    .label("No devices found")
                    .xalign(0.0)
                    .css_classes(vec!["qs-secondary"])
                    .build();
                empty_row.set_child(Some(&empty_label));
                list.append(&empty_row);
            } else {
                for device in &devs {
                    list.append(&create_bluetooth_row(device));
                }
            }

            if let Some(prev_mac) = previous {
                if let Some(idx) = devs.iter().position(|d| d.mac == prev_mac) {
                    if let Some(row) = list.row_at_index(idx as i32) {
                        list.select_row(Some(&row));
                    }
                } else {
                    list.unselect_all();
                }
            } else {
                list.unselect_all();
            }

            update_footer(selection, status_label, connect_btn, disconnect_btn, list, devices);
        }
        Ok(Err(err)) => {
            eprintln!("Failed to list bluetooth devices: {err}");
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            let error_row = ListBoxRow::new();
            let error_label = Label::builder()
                .label(&format!("Error: {err}"))
                .xalign(0.0)
                .css_classes(vec!["qs-secondary"])
                .build();
            error_row.set_child(Some(&error_label));
            list.append(&error_row);
            *selection.borrow_mut() = None;
            devices.borrow_mut().clear();
            update_footer(selection, status_label, connect_btn, disconnect_btn, list, devices);
        }
        Err(err) => {
            eprintln!("Failed to join bluetooth task: {err}");
        }
    }
}

fn create_bluetooth_row(device: &BluetoothDevice) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_selectable(true);
    row.set_activatable(true);

    let row_box = Box::new(Orientation::Horizontal, 12);
    row_box.set_margin_top(8);
    row_box.set_margin_bottom(8);
    row_box.set_margin_start(12);
    row_box.set_margin_end(12);

    let name_label = Label::builder()
        .label(&device.name)
        .xalign(0.0)
        .css_classes(vec!["qs-primary"])
        .build();
    row_box.append(&name_label);

    if device.connected {
        let status_label = Label::builder()
            .label("Connected")
            .xalign(0.0)
            .css_classes(vec!["qs-secondary"])
            .build();
        row_box.append(&status_label);
    }

    row.set_child(Some(&row_box));
    row
}

fn update_footer(
    selection: &Rc<RefCell<Option<String>>>,
    status_label: &Label,
    connect_btn: &Button,
    disconnect_btn: &Button,
    _list: &ListBox,
    devices: &Rc<RefCell<Vec<BluetoothDevice>>>,
) {
    connect_btn.set_label("Connect");
    disconnect_btn.set_label("Disconnect");

    if let Some(mac) = selection.borrow().clone() {
        let connected = devices
            .borrow()
            .iter()
            .find(|d| d.mac == mac)
            .map(|d| (d.connected, d.name.clone()));

        if let Some((connected, name)) = connected {
            let display_name = if name.is_empty() { mac.clone() } else { name };
            let label_text = if connected {
                format!("{display_name} connected")
            } else {
                display_name
            };
            status_label.set_label(&label_text);
            connect_btn.set_sensitive(!connected);
            disconnect_btn.set_sensitive(connected);
            return;
        }
    }

    status_label.set_label("Select a device");
    connect_btn.set_sensitive(false);
    disconnect_btn.set_sensitive(false);
}
