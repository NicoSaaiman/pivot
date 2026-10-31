pub fn convert_to_vec(arg: String, sep: char) -> Vec<String> {
    arg.split(sep).map(|x| x.to_string()).collect()
}

pub fn convert_kb_to_gb(value: f64) -> f64 {
    value / 1000.0 / 1000.0
}

pub fn convert_sec_to_min_and_sec(value: f64) -> (u64, u64) {
    let sec = (value % 60.0) as u64;
    let min = (value / 60.0) as u64;
    (min, sec)
}
