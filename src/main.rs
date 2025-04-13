use rustfft::{FftPlanner, num_complex::Complex};

use rerun::RecordingStream;
use rerun::external::image::metadata::Orientation;
use rerun::external::image::{DynamicImage, ImageBuffer, Rgb, Rgba};

mod audio_file;

const FLOAT_TO_COMPLEX: fn(&f64) -> Complex<f64> = |val| Complex::new(*val, 0.0);
const COMPLEX_TO_FLOAT: fn(&Complex<f64>) -> f64 = |val| val.re;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rec = rerun::RecordingStreamBuilder::new("music-man").spawn()?;
    println!("Music Man Started");
    let audio = read_file_and_log(&rec)?;

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
    let time_chunk_size: usize = 2048;
    // let freq_chunk_size = 32;
    // let slices_of_time = slice_time(&audio.sample_buffers[0], time_chunk_size);
    let mut chan = audio.sample_buffers[0].clone();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(time_chunk_size * 2); // Computes forward FFTs of size freq_chunk_size.

    // let mut collecting_chan = vec![];
    // for time in slices_of_time {
    let power_corrected_size =
        chan.len() + (time_chunk_size * 2) - (chan.len() % (time_chunk_size * 2));
    chan.resize(power_corrected_size, 0.0);

    let mut chan: Vec<_> = chan.iter().map(FLOAT_TO_COMPLEX).collect();
    fft.process(&mut chan);
    let times: Vec<Vec<Complex<f64>>> = chan
        .chunks_exact(time_chunk_size * 2)
        .map(|sl| sl[0..sl.len() / 2].to_vec())
        .collect();
    //     collecting_chan.push(chan[0..(chan.len() / 2)].to_vec());
    // }

    // Make sure I'm not insane.
    _ = times
        .iter()
        .map(|c| c.iter().map(|c| assert_eq!(c.re, c.im)));

    let chunked_left_chan: Vec<Vec<f64>> = times
        .iter()
        .map(|c| c.iter().map(COMPLEX_TO_FLOAT).collect()) // TODO: AHHH!!
        .collect();

    let sample_rate = audio.sample_rate;
    // ( sample_rate / time_chunk_size )
    println!("Sample rate: {}", sample_rate);

    render_spectro(&rec, chunked_left_chan, time_chunk_size as u32)?;

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
    rec: &RecordingStream,
    time_slices: Vec<Vec<f64>>,
    freq_domain: u32,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let max_freq = f64::ln(max_freq);
    let min_freq: f64 = {
        time_slices
            .clone() // TODO: STOP! PLEASE!
            .into_iter()
            .flatten()
            .reduce(f64::min)
            .unwrap()
    };
    println!("max freq : {}", max_freq);
    println!("min freq : {}", min_freq);

    for (index, freqs) in time_slices.iter().enumerate() {
        assert_eq!(freqs.len(), freq_domain as usize);
        let modded_freqs: Vec<[u8; 3]> = freqs
            .iter()
            .map(|f| {
                let val = ((f64::ln(*f) / max_freq) * 255.0) as u8;
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
    rec.log_static("image", &image)?;
    Ok(())
}

fn read_file_and_log(
    rec: &RecordingStream,
) -> Result<audio_file::AudioFile, Box<dyn std::error::Error>> {
    // let path = "2khz-sine.mp3";
    let path = "divine-service.mp3";
    let audio = audio_file::read_audio_file(path.to_string())?;

    println!("\nrecording...");
    // let rec = rerun::RecordingStreamBuilder::new("music-man").spawn()?;

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
