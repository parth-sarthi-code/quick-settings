use chrono::Local;

/// Get current time as formatted string
#[allow(dead_code)]
pub fn format_time() -> String {
    let now = Local::now();
    now.format("%H:%M").to_string()
}

/// Get current date as formatted string
#[allow(dead_code)]
pub fn format_date() -> String {
    let now = Local::now();
    now.format("%a %b %d").to_string()
}
