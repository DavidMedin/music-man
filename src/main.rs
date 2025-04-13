// use rustfft::{FftPlanner, num_complex::Complex};

use rerun::external::image::metadata::Orientation;
use rerun::external::image::{DynamicImage, ImageBuffer, Rgb, Rgba};
use rerun::external::re_types::blueprint::views::Spatial2DView;

mod audio_file;

// fn main() {
//     println!("Starting Music Analysis");

//     let mut planner = FftPlanner::new();
//     let size = 1234; // Computes forward FFTs of size 1234.
//     let fft = planner.plan_fft_forward(size);

//     let mut buffer = vec![
//         Complex {
//             re: 0.0f32,
//             im: 0.0f32
//         };
//         1234
//     ];
//     println!("buffer before : {:?}", buffer);
//     fft.process(&mut buffer);
//     println!("buffer after : {:?}", buffer);
// }
//

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rec = rerun::RecordingStreamBuilder::new("music-man-2").spawn()?;

    const FREQ_DOMAIN: usize = 100;
    let frequencies: Vec<[f64; FREQ_DOMAIN]> = {
        let mut cum = vec![];
        for i in 0..FREQ_DOMAIN {
            let mut buff = [0.0; FREQ_DOMAIN];
            buff[i] = 5.0;
            cum.push(buff);
        }
        cum
    };

    const MAX_FREQ: f64 = 5.0;

    const HEIGHT: u32 = 800;
    const WIDTH: u32 = FREQ_DOMAIN as u32;

    let mut image_data: Vec<u8> = Vec::new();
    image_data.resize((WIDTH * HEIGHT) as usize * 3, 0);

    for (index, freqs) in frequencies.iter().enumerate() {
        let modded_freqs: Vec<[u8; 3]> = freqs
            .iter()
            .map(|f| {
                let val = (f / MAX_FREQ) as u8 * 255;
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

fn read_file_and_log() -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}
