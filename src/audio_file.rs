use symphonia::core::audio::RawSampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioFile {
    pub path: String,
    pub sample_rate: u32,
    pub sample_buffers: [Vec<f64>; 2],
}

pub fn read_audio_file(path: String) -> Result<AudioFile, Box<dyn std::error::Error>> {
    // Open the media source.
    let src = std::fs::File::open(&path).expect("failed to open media");
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
    let sample_rate = track.codec_params.sample_rate.unwrap();

    let mut sample_buffs: [Vec<f64>; 2] = [vec![], vec![]];
    let mut scratch_buff = None;

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

                // Initialize buffers.
                if scratch_buff.is_none() {
                    // Get the audio buffer specification.
                    let spec = *decoded.spec();
                    // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                    let duration = decoded.capacity() as u64;

                    // Create the f64 sample buffer.
                    scratch_buff = Some(RawSampleBuffer::<f64>::new(duration, spec));
                }
                // ------------------

                if let Some(scratch) = scratch_buff.as_mut() {
                    scratch.copy_planar_ref(decoded);
                    assert_eq!(
                        scratch.len() % 2,
                        0,
                        "left and right channels are not equal in length!!!!"
                    );
                    let len = scratch.len();

                    // The samples may now be access via the `samples()` function.
                    let bytes = scratch.as_bytes();
                    let samples: Vec<f64> = bytes
                        .chunks_exact(8)
                        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
                        .collect();
                    assert_eq!(samples.len(), scratch.len());
                    let sample_parts: [&[f64]; 2] = samples.split_at(len / 2).into();
                    assert_eq!(sample_parts[0].len(), sample_parts[1].len());
                    for plane in 0..2 {
                        // left and right channels.
                        sample_buffs[plane].extend_from_slice(&sample_parts[plane]);
                    }

                    scratch.clear();
                } else {
                    unreachable!()
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

    assert_eq!(
        sample_buffs[0].len(),
        sample_buffs[1].len(),
        "Left and Right channels do not have same length!"
    );
    return Ok(AudioFile {
        path,
        sample_rate,
        sample_buffers: sample_buffs,
    });
}
