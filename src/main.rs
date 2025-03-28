// use rustfft::{FftPlanner, num_complex::Complex};

use std::collections::HashMap;

use symphonia::core::audio::{Channels, SampleBuffer, Signal};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::{Hint, ProbedMetadata};
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
pub struct Channel {
    sample_count: usize,
    sample_buff: SampleBuffer<f64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open the media source.
    let path = "/home/david/projects/music-analysis/divine-service-shorter.mp3";
    let src = std::fs::File::open(path).expect("failed to open media");
    println!("reading file : {}", path);

    // Create the media source stream.
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    // Create a probe hint using the file's extension. [Optional]
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    println!("meta_opts: {:?}", meta_opts);
    let fmt_opts: FormatOptions = Default::default();
    println!("fmt_opts: {:?}", fmt_opts);

    // Probe the media source.
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .expect("unsupported format");

    // let Some(metadata) = probed.metadata.get() else {
    //     panic!("no metadata");
    // };
    // metadata.

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known (decodeable) codec.
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");

    println!("found {} tracks", format.tracks().len());
    for track in format.tracks() {
        println!("got track {:?}", track);
    }

    // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();
    println!("dec_opts {:?}", dec_opts);

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("unsupported codec");

    // Store the track identifier, it will be used to filter packets.
    let track_id = track.id;

    let mut sample_count: usize = 0;
    let mut sample_buf = None;
    // let channels: HashMap<Channels, Channel> = HashMap::new();

    let rec = rerun::RecordingStreamBuilder::new("music-anal").spawn()?;

    // The decode loop.
    loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                // The track list has been changed. Re-examine it and create a new set of decoders,
                // then restart the decode loop. This is an advanced feature and it is not
                // unreasonable to consider this "the end." As of v0.5.0, the only usage of this is
                // for chained OGG physical streams.
                unimplemented!();
            }
            Err(Error::IoError(err)) => {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                }
                // A unrecoverable error occurred, halt decoding.
                panic!("{}", err);
            }
            Err(err) => {
                // A unrecoverable error occurred, halt decoding.
                panic!("{}", err);
            }
        };

        // Consume any new metadata that has been read since the last packet.
        while !format.metadata().is_latest() {
            // Pop the old head of the metadata queue.
            format.metadata().pop();

            // Consume the new metadata at the head of the metadata queue.
            let md = format.metadata();
            let front = md.current().unwrap();
            println!("metadata: {:?}", front);
        }

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != track_id {
            println!("skipping!");
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Consume the decoded audio samples (see below).
                if sample_buf.is_none() {
                    // Get the audio buffer specification.
                    let spec = *decoded.spec();
                    // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                    let duration = decoded.capacity() as u64;

                    // Create the f64 sample buffer.
                    sample_buf = Some(SampleBuffer::<f64>::new(duration, spec));
                }
                // Copy the decoded audio buffer into the sample buffer in an interleaved format.
                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(decoded);

                    // The samples may now be access via the `samples()` function.
                    // print!("\rDecoded {} samples", sample_count);

                    let samples = buf.samples();
                    for i in 0..samples.len() {
                        rec.set_time_sequence("step", i as i64 + sample_count as i64);
                        let value = &rerun::Scalar::new(samples[i] as f64);
                        // rec.log("scalars", value)?;
                        // rec.log("a log", &rerun::TextLog::new(format!("got {:?}", value)))?;
                        // print!("\rGot sample : {} at {}", samples[i], i);
                    }
                    sample_count += buf.samples().len();
                }
            }
            Err(Error::IoError(_)) => {
                // The packet failed to decode due to an IO error, skip the packet.
                println!("IO Error");
                continue;
            }
            Err(Error::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                println!("Decode error.");
                continue;
            }
            Err(err) => {
                // An unrecoverable error occurred, halt decoding.
                panic!("{}", err);
            }
        }
    }

    // println!("\nrecording...");
    println!("\nDone!");
    // for step in 0..64 {
    //     rec.set_time_sequence("step", step);
    //     rec.log("scalars", &rerun::Scalar::new((step as f64 / 10.0).sin()))?;
    // }

    Ok(())
}
