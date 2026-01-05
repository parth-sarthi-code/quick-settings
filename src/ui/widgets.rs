use chrono::Local;

/// Get current time as formatted string
pub fn current_time_string() -> String {
    let now = Local::now();
    now.format("%H:%M  %a %b %d").to_string()
}
