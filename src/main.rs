use rustfft::{FftPlanner, num_complex::Complex};

use rerun::RecordingStream;
use rerun::external::image::metadata::Orientation;
use rerun::external::image::{DynamicImage, ImageBuffer, Rgb, Rgba};
use rustfft::num_traits::pow;
use rustfft::num_traits::real::Real;

mod audio_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Music Man Started");
    let rec = rerun::RecordingStreamBuilder::new("music-man")
        // .recording_id("cs-d-out-of-tune")
        .spawn()?;
    let audio = read_file("2khz-sine.mp3".to_string())?;

    let time_chunk_base: usize = 512;
    for i in 0..1 {
        let time_chunk_size = time_chunk_base * 2usize.pow(i as u32);

        let left_chan_results =
            fft_samples(&audio.sample_buffers[0], time_chunk_size, audio.sample_rate)?;

        let spectro = Spectrogram::new(
            left_chan_results.freqs,
            time_chunk_size as u32,
            left_chan_results.hz_per_element,
        )?;

        let title = format!("{}-freqs", time_chunk_size);
        log_spectrogram(&rec, &spectro, &title)?;
    }
    Ok(())
}

struct FftResult {
    freqs: Vec<Vec<f64>>,
    hz_per_element: f64,
}

fn fft_samples(
    samples: &Vec<f64>,
    time_chunk_size: usize,
    sample_rate: u32,
) -> Result<FftResult, Box<dyn std::error::Error>> {
    let mut samples = samples.clone();

    let take_one_over_n: usize = 8; // extracted for potential future.
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(time_chunk_size * take_one_over_n); // Computes forward FFTs of size freq_chunk_size.

    // let power_corrected_size = samples.len() + (time_chunk_size * take_one_over_n)
    //     - (samples.len() % (time_chunk_size * take_one_over_n));
    let chunk_count = samples.len().div_ceil(time_chunk_size);
    samples.resize(chunk_count * time_chunk_size, 0.0);
    let chunk_jump = time_chunk_size / take_one_over_n;
    // let chunk_count = chunk_count * take_one_over_n - (take_one_over_n - 1); // add the chunks that are between consecutive chunks, and remove all chunks that would reach after the resized buffer.
    // let chunk_jump = time_chunk_size / take_one_over_n;

    const FLOAT_TO_COMPLEX: fn(&f64) -> Complex<f64> = |val| Complex::new(*val, 0.0);
    const COMPLEX_TO_FLOAT: fn(&Complex<f64>) -> f64 = |val| val.re + val.im;
    let mut complex_samples: Vec<_> = samples.iter().map(FLOAT_TO_COMPLEX).collect();

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

    // Remove the top half of each column (of the spectrogram). Effectively taking the lower half.
    // I think it's because the top half is less precise? Not sure abt that one.
    // let complex_samples: Vec<Vec<Complex<f64>>> = complex_samples
    //     .chunks_exact(time_chunk_size * take_one_over_n)
    //     .map(|sl| sl[0..sl.len() / take_one_over_n].to_vec())
    //     .collect();

    // Make sure I'm not insane.
    // _ = complex_samples
    //     .iter()
    //     .map(|c| c.iter().map(|c| assert_eq!(c.re, c.im)));

    let float_samples: Vec<Vec<f64>> = output
        .iter()
        .map(|c| c.iter().map(COMPLEX_TO_FLOAT).collect()) // TODO: AHHH!!
        .collect();

    let rez = FftResult {
        freqs: float_samples,
        hz_per_element: sample_rate as f64 / (time_chunk_size * take_one_over_n) as f64,
    };
    Ok(rez)
}

pub struct Spectrogram {
    rgb_buffer: Vec<u8>,
    size_px: [u32; 2],
    freq_per_px: f64,
}

impl Spectrogram {
    pub fn new(
        time_slices: Vec<Vec<f64>>,
        freq_domain: u32,
        freq_per_px: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let height: u32 = time_slices.len() as u32;
        let width: u32 = freq_domain;

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
        let spect = Spectrogram {
            rgb_buffer: image_data,
            size_px: [width, height],
            freq_per_px,
        };
        Ok(spect)
    }
    pub fn freq_to_position(&self, freq: f64) -> f64 {
        freq / self.freq_per_px
    }
}

fn log_spectrogram(
    rec: &RecordingStream,
    spectro: &Spectrogram,
    log_entity: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let [mut width, mut height] = spectro.size_px;
    let image_data = spectro.rgb_buffer.clone();

    let image_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, image_data).unwrap();
    let mut dyn_image = DynamicImage::ImageRgb8(image_buffer);
    dyn_image.apply_orientation(Orientation::Rotate270);
    let image = rerun::Image::from_dynamic_image(dyn_image)?;

    std::mem::swap(&mut width, &mut height); // since we have rotated the image.

    rec.log_static(log_entity.clone(), &image)?;

    let origin = [0.0, height as f32];
    // let D_note_freq = 2000.0;
    let D_note_freq = 146.83;
    // let D_note_freq = spectro.freq_per_px;

    let arrow_size = 44.0;

    let mut origins = vec![origin, origin];
    let mut arrow_vecs = vec![[arrow_size, 0.0], [0.0, -arrow_size]];
    let mut arrow_colors = vec![[255, 0, 0], [0, 255, 0]];
    let mut arrow_labels = vec!["time".to_string(), "freq".to_string()];

    let d_note_color = [255, 0, 0];

    for i in 0..4 {
        let freq = D_note_freq * pow(2.0, i);
        let D_note_px = (height as f64 - spectro.freq_to_position(freq)) as f32;
        origins.push([0.0, D_note_px + 0.5]); // +0.5 to point to the middle of the pixel it references.
        arrow_vecs.push([arrow_size / 3.0, 0.0]);
        arrow_colors.push(d_note_color);
        arrow_labels.push(format!("{} Hz", freq));
    }

    rec.log_static(
        format!("/{}/arrows", log_entity),
        &rerun::Arrows2D::from_vectors(arrow_vecs)
            .with_radii([0.25])
            .with_origins(origins)
            .with_colors(arrow_colors)
            .with_labels(arrow_labels),
    )?;

    Ok(())
}

fn read_file(path: String) -> Result<audio_file::AudioFile, Box<dyn std::error::Error>> {
    let audio = audio_file::read_audio_file(path)?;

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
