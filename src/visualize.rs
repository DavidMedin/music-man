use crate::{audio_file, audio_process::FftResult};
use rerun::external::image::metadata::Orientation;
use rerun::external::image::{DynamicImage, ImageBuffer, Rgb};
use rerun::{RecordingStream, Scalar};
use rustfft::num_traits::pow;

pub struct Spectrogram {
    pub rgb_buffer: Vec<u8>,
    pub size_px: [u32; 2],
    pub freq_per_px: f64,
}

impl Spectrogram {
    pub fn new(
        time_slices: &Vec<Vec<f64>>,
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

/// # Warning! This will send a ton of data to the viewer, consuming a lot of memory!
/// A normally sized song will end up being ~1GiB of data to the viewer!
pub fn log_audio_file(
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

pub fn log_spectrogram(
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
    let d_note_freq = 146.83;

    let arrow_size = 44.0;

    let mut origins = vec![origin, origin];
    let mut arrow_vecs = vec![[arrow_size, 0.0], [0.0, -arrow_size]];
    let mut arrow_colors = vec![[255, 0, 0], [0, 255, 0]];
    let mut arrow_labels = vec!["time".to_string(), "freq".to_string()];

    let d_note_color = [255, 0, 0];

    for i in 0..4 {
        let freq = d_note_freq * pow(2.0, i);
        let d_note_px = (height as f64 - spectro.freq_to_position(freq)) as f32;
        origins.push([0.0, d_note_px + 0.5]); // +0.5 to point to the middle of the pixel it references.
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

pub fn log_freq_time_plot(
    rec: &RecordingStream,
    fft_result: &FftResult,
    freq: f64,
    title: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_idx = (freq / fft_result.hz_per_element) as usize;
    let samples: Vec<f64> = fft_result.freqs.iter().map(|hzs| hzs[target_idx]).collect();

    for (idx, sample) in samples.iter().enumerate() {
        rec.set_time_sequence("time", idx as i64);
        rec.log(title.as_str(), &Scalar::new(*sample))?;
    }

    Ok(())
}
