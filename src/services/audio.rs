use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

use super::{AudioOutput, AudioState};

/// Audio service backed on wpctl (PipeWire/WirePlumber control).
#[allow(dead_code)]
#[derive(Clone)]
pub struct AudioService {
    state: Arc<RwLock<AudioState>>,
}

impl AudioService {
    /// Create audio service.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AudioState {
                volume: 0.0,
                is_muted: false,
                volume_percent: 0,
                default_output: None,
                outputs: Vec::new(),
            })),
        }
    }

    /// Get audio outputs from wpctl status.
    #[allow(dead_code)]
    pub async fn outputs(&self) -> Result<Vec<AudioOutput>> {
        let list = self.query_outputs().await?;
        let mut state = self.state.write().await;
        state.outputs = list.clone();
        state.default_output = list.iter().find(|o| o.is_default).map(|o| o.id.clone());
        Ok(list)
    }

    /// Set default output device using wpctl.
    #[allow(dead_code)]
    pub async fn set_default_output(&self, id: &str) -> Result<()> {
        let status = Command::new("wpctl")
            .arg("set-default")
            .arg(id)
            .status()
            .await
            .context("run wpctl set-default")?;

        if !status.success() {
            return Err(anyhow!("wpctl set-default {id} failed"));
        }

        // Refresh outputs to update default marker
        let _ = self.outputs().await;
        Ok(())
    }

    /// Set volume (stub - not yet implemented).
    #[allow(dead_code)]
    pub async fn set_volume(&self, volume: f64) -> Result<()> {
        let volume = volume.clamp(0.0, 1.0);
        let percent = (volume * 100.0) as u32;
        let mut state = self.state.write().await;
        state.volume = volume;
        state.volume_percent = percent;
        Ok(())
    }

    #[allow(dead_code)]
    async fn query_outputs(&self) -> Result<Vec<AudioOutput>> {
        let output = Command::new("wpctl")
            .arg("status")
            .output()
            .await
            .context("run wpctl status")?;

        if !output.status.success() {
            return Err(anyhow!("wpctl status failed"));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        Ok(parse_wpctl_status(&text))
    }
}

#[allow(dead_code)]
fn shorten_device_name(name: &str) -> String {
    let name = name.trim();

    // Remove common hardware controller prefixes that repeat across multiple outputs
    let common_prefixes = [
        "Raptor Lake High Definition Audio Controller ",
        "Intel High Definition Audio Controller ",
        "High Definition Audio Controller ",
        "PCI ",
    ];

    let mut clean_name = name.to_string();
    for prefix in &common_prefixes {
        if clean_name.starts_with(prefix) {
            clean_name = clean_name[prefix.len()..].to_string();
            break;
        }
    }

    // Extract just the port info for HDMI/DisplayPort
    if let Some(pos) = clean_name.rfind("HDMI") {
        return clean_name[pos..].to_string();
    }
    if let Some(pos) = clean_name.rfind("DisplayPort") {
        return clean_name[pos..].to_string();
    }

    // For Bluetooth devices, keep the name as is
    if name.contains("bluez") || name.to_lowercase().contains("bluetooth") {
        return clean_name;
    }

    // For other devices (like Speaker), limit to 35 characters
    if clean_name.len() > 35 {
        format!("{}...", &clean_name[..32])
    } else {
        clean_name
    }
}

#[allow(dead_code)]
fn parse_wpctl_status(text: &str) -> Vec<AudioOutput> {
    let mut in_sinks = false;
    let mut in_filters = false;
    let mut outputs = Vec::new();
    let mut default_id: Option<String> = None;
    let mut filter_sinks = std::collections::HashMap::new();

    for line in text.lines() {
        // Section markers
        if line.contains("Sinks:") && !line.contains("Default") {
            in_sinks = true;
            in_filters = false;
            continue;
        }
        if line.contains("Sources:") {
            in_sinks = false;
            continue;
        }
        if line.contains("Filters:") {
            in_sinks = false;
            in_filters = true;
            continue;
        }
        if line.contains("Streams:") || line.contains("Settings:") {
            in_sinks = false;
            in_filters = false;
        }

        if in_sinks {
            parse_sink_line(line, &mut outputs, &mut default_id);
        } else if in_filters {
            parse_filter_sink_line(line, &mut filter_sinks, &mut default_id);
        }
    }

    // Replace Bluetooth device IDs with their Filter node IDs
    replace_bluetooth_ids(&mut outputs, &filter_sinks);

    // Set default if found
    if let Some(ref def_id) = default_id {
        for output in &mut outputs {
            output.is_default = output.id == *def_id;
        }
    } else if !outputs.is_empty() {
        outputs[0].is_default = true;
    }

    outputs
}

#[allow(dead_code)]
fn parse_sink_line(line: &str, outputs: &mut Vec<AudioOutput>, default_id: &mut Option<String>) {
    let trimmed = line.trim_start();

    if trimmed.is_empty() || !trimmed.starts_with('│') {
        return;
    }

    let content = trimmed.trim_start_matches('│').trim_start();
    let is_default = content.starts_with('*');
    let content = if is_default {
        content.trim_start_matches('*').trim_start()
    } else {
        content
    };

    if let Some(dot_idx) = content.find('.') {
        let id_str = content[..dot_idx].trim();
        let rest = content[dot_idx + 1..].trim();

        let name = if let Some(bracket) = rest.find('[') {
            rest[..bracket].trim()
        } else {
            rest
        };

        if !id_str.is_empty() && id_str.chars().all(|c| c.is_ascii_digit()) && !name.is_empty() {
            outputs.push(AudioOutput {
                id: id_str.to_string(),
                name: shorten_device_name(name),
                is_default,
            });

            if is_default {
                *default_id = Some(id_str.to_string());
            }
        }
    }
}

#[allow(dead_code)]
fn parse_filter_sink_line(
    line: &str,
    filter_sinks: &mut std::collections::HashMap<String, String>,
    default_id: &mut Option<String>,
) {
    if !line.contains("[Audio/Sink]") {
        return;
    }

    let trimmed = line.trim_start();
    if trimmed.is_empty() || !trimmed.starts_with('│') {
        return;
    }

    let content = trimmed.trim_start_matches('│').trim_start();
    let is_default = content.starts_with('*');
    let content = if is_default {
        content.trim_start_matches('*').trim_start()
    } else {
        content
    };

    if let Some(dot_idx) = content.find('.') {
        let id_str = content[..dot_idx].trim();
        let rest = content[dot_idx + 1..].trim();

        let node_name = if let Some(bracket) = rest.find('[') {
            rest[..bracket].trim()
        } else {
            rest
        };

        if !id_str.is_empty() && id_str.chars().all(|c| c.is_ascii_digit()) {
            filter_sinks.insert(node_name.to_string(), id_str.to_string());
            if is_default {
                *default_id = Some(id_str.to_string());
            }
        }
    }
}

#[allow(dead_code)]
fn replace_bluetooth_ids(
    outputs: &mut Vec<AudioOutput>,
    filter_sinks: &std::collections::HashMap<String, String>,
) {
    for output in outputs {
        if output.name.contains("soundcore") || output.name.contains("bluez") {
            for (node_name, filter_id) in filter_sinks {
                if node_name.contains("bluez_output") {
                    output.id = filter_id.clone();
                    break;
                }
            }
        }
    }
}
