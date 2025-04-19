mod audio_file;
mod audio_process;
mod visualize;

use audio_file::read_audio_file;
use audio_process::fft_samples;
use visualize::{Spectrogram, log_freq_time_plot, log_spectrogram};

// TODO: look into these links
/*
https://www.chciken.com/digital/signal/processing/2020/05/13/guitar-tuner.html
https://en.wikipedia.org/wiki/Nyquist%E2%80%93Shannon_sampling_theorem
https://www.chciken.com/digital/signal/processing/2020/04/13/dft.html
https://en.wikipedia.org/wiki/Pitch_detection_algorithm
https://en.wikipedia.org/wiki/Spectral_leakage

*/

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Music Man Started");
    let rec = rerun::RecordingStreamBuilder::new("music-man")
        // .recording_id("cs-d-out-of-tune")
        .spawn()?;
    let audio = read_audio_file("samples/in-tune-D.mp3".to_string())?;

    let time_chunk_base: usize = 512;
    for i in 0..1 {
        let time_chunk_size = time_chunk_base * 2usize.pow(i as u32);
        println!("time_chunk_size: {}", time_chunk_size);

        let left_chan_results =
            fft_samples(&audio.sample_buffers[0], time_chunk_size, audio.sample_rate)?;

        let title = format!("{}-amps", time_chunk_size);
        log_freq_time_plot(&rec, &left_chan_results, 2000.0, &title)?;
        let spectro = Spectrogram::new(
            &left_chan_results.freqs,
            time_chunk_size as u32,
            left_chan_results.hz_per_element,
        )?;

        let title = format!("{}-freqs", time_chunk_size);
        log_spectrogram(&rec, &spectro, &title)?;
    }
    Ok(())
}
