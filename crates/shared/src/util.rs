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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_to_vec_success() {
        let result = convert_to_vec("one,two,three".to_string(), ',');

        assert_eq!(
            result, 
            vec![
                "one".to_string(), 
                "two".to_string(), 
                "three".to_string()
            ]
        );
    }

    #[test]
    fn convert_to_vec_empty() {
        let result = convert_to_vec("".to_string(), ',');

        assert_eq!(result, vec!["".to_string()]);
    }
}
