pub fn fmt_size(u: usize) -> String {
    let mut u = u as f64;
    let mut i = 0;
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    while u >= 1024. && i < units.len() - 1 {
        u /= 1024.;
        i += 1;
    }
    format!("{:.2} {}", u, units[i])
}
