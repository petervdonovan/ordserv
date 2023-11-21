#[derive(Debug)]
pub struct BasicStats {
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub upper_quartile: f64,
    pub lower_quartile: f64,
}
pub type StatProjection = (String, Box<dyn Fn(&BasicStats) -> f64>);
impl BasicStats {
    pub fn new(data: impl Iterator<Item = f64>) -> Self {
        let mut data: Vec<_> = data.collect();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = data.len();
        let mean = data.iter().sum::<f64>() / len as f64;
        let median = data[len / 2];
        let min = data[0];
        let max = data[len - 1];
        let upper_quartile = data[(len * 3) / 4];
        let lower_quartile = data[len / 4];
        Self {
            mean,
            median,
            min,
            max,
            upper_quartile,
            lower_quartile,
        }
    }
    pub fn projections() -> Vec<StatProjection> {
        vec![
            ("Mean".to_string(), Box::new(|it: &Self| it.mean)),
            ("Median".to_string(), Box::new(|it| it.median)),
            ("Minimum".to_string(), Box::new(|it| it.min)),
            ("Maximum".to_string(), Box::new(|it| it.max)),
            (
                "Upper Quartile".to_string(),
                Box::new(|it| it.upper_quartile),
            ),
            (
                "Lower Quartile".to_string(),
                Box::new(|it| it.lower_quartile),
            ),
        ]
    }
}
