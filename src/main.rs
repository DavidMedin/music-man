use rustfft::{FftPlanner, num_complex::Complex};

use rerun::RecordingStream;
use rerun::external::image::metadata::Orientation;
use rerun::external::image::{DynamicImage, ImageBuffer, Rgb, Rgba};

mod audio_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rec = rerun::RecordingStreamBuilder::new("music-man").spawn()?;
    println!("Music Man Started");
    let audio = read_file()?;

    println!("Starting Music Analysis");

    let time_chunk_size: usize = 2048;
    let left_chan_freqs = fft_samples(&audio.sample_buffers[0], time_chunk_size)?;

    let sample_rate = audio.sample_rate;
    println!("Sample rate: {}", sample_rate);

    // ( sample_rate / time_chunk_size )

    render_spectro(&rec, left_chan_freqs, time_chunk_size as u32)?;
    Ok(())
}

fn fft_samples(
    samples: &Vec<f64>,
    time_chunk_size: usize,
) -> Result<Vec<Vec<f64>>, Box<dyn std::error::Error>> {
    let mut samples = samples.clone();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(time_chunk_size * 2); // Computes forward FFTs of size freq_chunk_size.

    let power_corrected_size =
        samples.len() + (time_chunk_size * 2) - (samples.len() % (time_chunk_size * 2));
    samples.resize(power_corrected_size, 0.0);

    const FLOAT_TO_COMPLEX: fn(&f64) -> Complex<f64> = |val| Complex::new(*val, 0.0);
    const COMPLEX_TO_FLOAT: fn(&Complex<f64>) -> f64 = |val| val.re;
    let mut complex_samples: Vec<_> = samples.iter().map(FLOAT_TO_COMPLEX).collect();

    fft.process(&mut complex_samples); // Big math

    // Remove the top half of each column (of the spectrogram). Effectively taking the lower half.
    // I think it's because the top half is less precise? Not sure abt that one.
    let complex_samples: Vec<Vec<Complex<f64>>> = complex_samples
        .chunks_exact(time_chunk_size * 2)
        .map(|sl| sl[0..sl.len() / 2].to_vec())
        .collect();

    // Make sure I'm not insane.
    _ = complex_samples
        .iter()
        .map(|c| c.iter().map(|c| assert_eq!(c.re, c.im)));

    let float_samples: Vec<Vec<f64>> = complex_samples
        .iter()
        .map(|c| c.iter().map(COMPLEX_TO_FLOAT).collect()) // TODO: AHHH!!
        .collect();
    Ok(float_samples)
}

fn render_spectro(
    rec: &RecordingStream,
    time_slices: Vec<Vec<f64>>,
    freq_domain: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut height: u32 = time_slices.len() as u32;
    let mut width: u32 = freq_domain;

    let mut image_data: Vec<u8> = Vec::new();
    image_data.resize((width * height) as usize * 3, 0);

    let time_slices: Vec<Vec<f64>> = time_slices
        .into_iter()
        .map(|sl| sl.iter().map(|v| v.abs()).collect())
        .collect();
    let max_freq: f64 = {
        time_slices
            .clone() // TODO: STOP! PLEASE!
            .into_iter()
            .flatten()
            .reduce(f64::max)
            .unwrap()
    };

    let mod_fn = |x| x;
    // let mod_fn = |x| f64::ln(x);
    let max_freq = mod_fn(max_freq);

    for (index, freqs) in time_slices.iter().enumerate() {
        assert_eq!(freqs.len(), freq_domain as usize);
        let modded_freqs: Vec<[u8; 3]> = freqs
            .iter()
            .map(|f| {
                let val = ((mod_fn(*f) / max_freq) * 255.0) as u8;
                [val, val, val]
            })
            .collect();
        let flat = modded_freqs.as_flattened();
        let line_start = 3 * index * width as usize;
        let line_end = 3 * (index + 1) * width as usize;
        (&mut image_data[line_start..line_end]).copy_from_slice(flat);
    }

    let image_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, image_data).unwrap();
    let mut dyn_image = DynamicImage::ImageRgb8(image_buffer);
    dyn_image.apply_orientation(Orientation::Rotate270);
    let image = rerun::Image::from_dynamic_image(dyn_image)?;
    std::mem::swap(&mut width, &mut height); // since we have rotated the image.
    rec.log_static("image", &image)?;

    let origin = [0.0, height as f32];
    let origins = [origin; 2];

    let arrow_size = 44.0;
    rec.log_static(
        "/image/arrows",
        &rerun::Arrows2D::from_vectors([[arrow_size, 0.0], [0.0, -arrow_size]])
            .with_radii([0.25])
            .with_origins(origins)
            .with_colors([[255, 0, 0], [0, 255, 0]])
            .with_labels(["time", "freq"]),
    )?;

    Ok(())
}

fn read_file() -> Result<audio_file::AudioFile, Box<dyn std::error::Error>> {
    // let path = "2khz-sine.mp3";
    let path = "samples/d-note-tuned.mp3";
    let audio = audio_file::read_audio_file(path.to_string())?;

    Ok(audio)
}

/// # Warning! This will send a ton of data to the viewer, consuming a lot of memory!
/// A normally sized song will end up being ~1GiB of data to the viewer!
fn log_audio_file(
    rec: &RecordingStream,
    audio: &audio_file::AudioFile,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nrecording...");

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
    Ok(())
}
