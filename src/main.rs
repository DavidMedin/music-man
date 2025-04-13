use rustfft::{FftPlanner, num_complex::Complex};

use rerun::external::image::metadata::Orientation;
use rerun::external::image::{DynamicImage, ImageBuffer, Rgb, Rgba};
use rerun::external::re_types::blueprint::views::Spatial2DView;

mod audio_file;

// const MAX_FREQ: f64 = (20.0 * 1000.0) / 2.0; // 20kHz (max human hearing)
// const FREQ_DOMAIN: usize = 800;
const FLOAT_TO_COMPLEX: fn(&f64) -> Complex<f64> = |val| Complex::new(*val, 0.0);
const COMPLEX_TO_FLOAT: fn(&Complex<f64>) -> f64 = |val| val.im;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Music Man Started");
    let audio = read_file_and_log()?;

    // TODO: Don't clone
    println!(
        "before max sample : {}",
        audio.sample_buffers[0]
            .clone()
            .into_iter()
            .reduce(f64::max)
            .unwrap()
    );

    println!("Starting Music Analysis");
    // TODO: make it so that time doesn't need to be a multiple of freq.
    let time_chunk_size: u32 = 800;
    let freq_chunk_size = 100;
    let slices_of_time = slice_time(&audio.sample_buffers[0], time_chunk_size);

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(freq_chunk_size); // Computes forward FFTs of size freq_chunk_size.

    let mut collecting_chan = vec![];
    for time in slices_of_time {
        let mut chan: Vec<_> = time.iter().map(FLOAT_TO_COMPLEX).collect();
        for chunk in chan.chunks_exact_mut(freq_chunk_size) {
            fft.process(chunk);
        }
        collecting_chan.push(chan);
    }
    let chunked_left_chan: Vec<Vec<f64>> = collecting_chan
        .iter()
        .map(|c| c.iter().map(COMPLEX_TO_FLOAT).collect()) // TODO: AHHH!!
        .collect();
    render_spectro(chunked_left_chan, time_chunk_size)?;

    // Example spectrogram.
    // let frequencies: Vec<[f64; FREQ_DOMAIN]> = {
    //     let mut cum = vec![];
    //     for i in 0..FREQ_DOMAIN {
    //         let mut buff = [0.0; FREQ_DOMAIN];
    //         buff[i] = 5.0;
    //         cum.push(buff);
    //     }
    //     cum
    // };

    Ok(())
}

// let slices_of_time = slice_time(arr)
// let freqs_over_time = slices_of_time.iter().map(|t| fft.process(t))

fn slice_time(audio_sample: &Vec<f64>, chunk_size: u32) -> Vec<&[f64]> {
    // TODO: Don't throw away misaligned chunks.
    audio_sample.chunks_exact(chunk_size as usize).collect()
}

fn render_spectro(
    frequencies: Vec<Vec<f64>>,
    freq_domain: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let rec = rerun::RecordingStreamBuilder::new("music-man").spawn()?;

    const HEIGHT: u32 = 800;
    let WIDTH: u32 = freq_domain;

    let mut image_data: Vec<u8> = Vec::new();
    image_data.resize((WIDTH * HEIGHT) as usize * 3, 0);

    let max_freq: f64 = {
        frequencies
            .clone() // TODO: STOP! PLEASE!
            .into_iter()
            .flatten()
            .reduce(f64::max)
            .unwrap()
    };
    println!("max freq : {}", max_freq);

    for (index, freqs) in frequencies.iter().enumerate() {
        let modded_freqs: Vec<[u8; 3]> = freqs
            .iter()
            .map(|f| {
                let val = (f / max_freq) as u8 * 255;
                [val, val, val]
            })
            .collect();
        let flat = modded_freqs.as_flattened();
        let line_start = 3 * index * WIDTH as usize;
        let line_end = 3 * (index + 1) * WIDTH as usize;
        (&mut image_data[line_start..line_end]).copy_from_slice(flat);
    }

    let image_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(WIDTH, HEIGHT, image_data).unwrap();
    let mut dyn_image = DynamicImage::ImageRgb8(image_buffer);
    dyn_image.apply_orientation(Orientation::Rotate270);
    let image = rerun::Image::from_dynamic_image(dyn_image)?;
    rec.log_static("image", &image)?;
    Ok(())
}

fn read_file_and_log() -> Result<audio_file::AudioFile, Box<dyn std::error::Error>> {
    let path = "divine-service-shorter.mp3";
    let audio = audio_file::read_audio_file(path.to_string())?;

    println!("\nrecording...");
    let rec = rerun::RecordingStreamBuilder::new("music-man").spawn()?;

    for chan in 0..2 {
        let buf = &audio.sample_buffers[chan];
        for index in 0..audio.sample_buffers[chan].len() {
            rec.set_time_seconds(
                "step",
                (index as i64 + buf.len() as i64) as f64 / (audio.sample_rate as f64),
            );
            let value = &rerun::Scalar::new(buf[index]);
            rec.log(format!("channel {}", chan), value)?;
        }
    }
    println!("\nDone!");

    Ok(audio)
}
