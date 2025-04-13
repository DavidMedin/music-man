// use rustfft::{FftPlanner, num_complex::Complex};

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
    // let view = Spatial2DView {
    //     background: Default::default(),
    //     visual_bounds: Default::default(),
    //     time_ranges: Default::default(),
    // };

    let rec = rerun::RecordingStreamBuilder::new("music-man").spawn()?;

    // Define the component.
    let width = 800;
    let height = 800;
    let img_fmt = rerun::components::ImageFormat::rgb8([width, height]);

    let mut image_data: Vec<u8> = Vec::new();
    image_data.resize((width * height) as usize * 3, 100);

    // rec.log("image_ent", img_fmt)
    let image = rerun::Image::update_fields()
        .with_format(img_fmt)
        .with_buffer(image_data);
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
