mod ipc;
mod services;
mod state;
mod ui;
mod utils;

use anyhow::Result;
use gtk4::prelude::*;
use gtk4::{glib, Application};
use std::sync::Arc;
use tokio::sync::RwLock;

use state::AppState;
use ui::quick_settings::QuickSettings;

const APP_ID: &str = "com.niri.bar";

fn main() -> Result<()> {
    // Initialize GTK
    gtk4::init()?;

    // Create the GTK application
    let app = Application::builder().application_id(APP_ID).build();

    // Create shared state
    let state = Arc::new(RwLock::new(AppState::new()));

    // Clone for closures
    let state_clone = Arc::clone(&state);

    app.connect_activate(move |app| {
        // Build the quick settings panel (hidden by default)
        let quick_settings = QuickSettings::new(app, Arc::clone(&state_clone));

        // Setup signal handler for SIGUSR1 to toggle panel
        let qs_clone = Arc::clone(&quick_settings);
        glib::unix_signal_add_local(10, move || {
            // SIGUSR1 = 10 on Unix
            let qs = Arc::clone(&qs_clone);
            glib::spawn_future_local(async move {
                qs.toggle().await;
            });
            glib::ControlFlow::Continue
        });

        // Don't show initially - waybar will trigger it
        // quick_settings.show();
    });

    // Start the async runtime in a separate thread
    let state_async = Arc::clone(&state);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Start IPC listener
            if let Err(e) = ipc::niri::start_listener(state_async).await {
                eprintln!("IPC listener error: {}", e);
            }
        });
    });

    // Run the GTK main loop
    app.run();

    Ok(())
}
