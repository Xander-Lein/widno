pub const HEADER_HEIGHT: f32 = 20.0;

pub fn format_metadata(
    name: &str,
    dimensions: (u32, u32),
    bytes: u64,
    position: &str,
    zoom: f32,
) -> String {
    format!(
        "{}  |  {}×{}  |  {}  |  {}  |  zoom {:.0}%",
        name,
        dimensions.0,
        dimensions.1,
        format_bytes(bytes),
        position,
        zoom * 100.0
    )
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
