use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

pub struct FftResult {
    pub freqs: Vec<Vec<f64>>,
    pub hz_per_element: f64,
}
pub fn fft_samples(
    samples: &Vec<f64>,
    time_chunk_size: usize,
    sample_rate: u32,
) -> Result<FftResult, Box<dyn std::error::Error>> {
    let mut samples = samples.clone();

    let take_one_over_n: usize = 8; // extracted for potential future.
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(time_chunk_size * take_one_over_n); // Computes forward FFTs of size freq_chunk_size.

    let chunk_count = samples.len().div_ceil(time_chunk_size);
    samples.resize(chunk_count * time_chunk_size, 0.0);
    let chunk_jump = time_chunk_size / take_one_over_n;

    println!("chunk_count: {}", chunk_count);
    println!("chunk_jump: {}", chunk_jump);
    println!("sample len: {}", samples.len());
    const FLOAT_TO_COMPLEX: fn(&f64) -> Complex<f64> = |val| Complex::new(*val, 0.0);
    const MAG_OF_COMPLEX: fn(&Complex<f64>) -> f64 = |val| (val.re.powi(2) + val.im.powi(2)).sqrt();
    let complex_samples: Vec<_> = samples.iter().map(FLOAT_TO_COMPLEX).collect();

    let mut output = vec![];
    let mut sample_num = 0;
    while sample_num + time_chunk_size * take_one_over_n < complex_samples.len() {
        let mut input_buff =
            complex_samples[sample_num..sample_num + time_chunk_size * take_one_over_n].to_vec();
        fft.process(&mut input_buff); // Big math
        let cut_off_a_bunch = input_buff[..input_buff.len() / take_one_over_n].to_vec();
        output.push(cut_off_a_bunch);
        sample_num += chunk_jump;
    }

    let float_samples: Vec<Vec<f64>> = output
        .iter()
        .map(|c| c.iter().map(MAG_OF_COMPLEX).collect()) // TODO: AHHH!!
        .collect();

    let rez = FftResult {
        freqs: float_samples,
        hz_per_element: sample_rate as f64 / (time_chunk_size * take_one_over_n) as f64,
    };
    Ok(rez)
}

// TODO: make this like 100 times better pls
pub fn get_fundamental_frequency(freqs: &Vec<f64>, hz_per_element: f64) -> f64 {
    let (huh, what) =
        freqs.iter().enumerate().fold(
            (0, 0.0),
            |(a_idx, a), (b_idx, &b)| if a < b { (b_idx, b) } else { (a_idx, a) },
        );

    hz_per_element * (huh as f64)
}
