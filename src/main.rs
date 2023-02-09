use std::{fs::File, time::Duration};
use std::io::BufReader;
use rodio::Sink;
use rodio::source::{SineWave, Buffered};
use rodio::{Decoder, OutputStream, source::Source};
use itertools::MultiPeek;
use rtrb::Consumer;
use rustfft::algorithm::Radix4;
use rustfft::{FftDirection, FftPlanner};
use rustfft::num_complex::Complex;

fn main() {
    // Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open("magic-escape-room.mp3").unwrap());
    // Decode that sound file into a source
    let source = Decoder::new(file).unwrap();

    let source = SineWave::new(80.0);

    // Play the sound directly on the device
    let sample_rate = source.sample_rate();
    
    let (mut producer, mut consumer) = rtrb::RingBuffer::<Buffered<_>>::new(1);
    std::thread::spawn(move || {
        fft_check(consumer, sample_rate);
    });


    let source = source
        .convert_samples()
        .buffered()
        .periodic_access(Duration::from_millis(1000), move |s| {
        let s = s.clone();
        producer.push(s);
        // let s : u32 = s.fold(0, |acc : u32, x : i16| acc.saturating_add(x.unsigned_abs() as u32));
        // println!("{s}")
        // producer.push(*s);
    });


    let sink = Sink::try_new(&stream_handle).unwrap();

    sink.append(source);
    sink.sleep_until_end();
}


fn crude_amp_check<I: Iterator<Item = f32>>(mut consumer : Consumer<I>) -> ! {
    loop {
        if let Ok(iter) = consumer.pop() {
            let (mut max, mut min) = (0f32, 0f32);
            for t in iter.take(100) {
                if t < min {
                    min = t;
                } else if t > max {
                    max = t;
                }
            }
            let delta = max - min;
            if delta > 0.1 {
                println!("{delta:#?}");
            }
        };
    }
}

fn fft_check<I : Source<Item = f32>>(mut consumer: Consumer<I>, sample_rate: u32) -> ! {
    let sample_size = sample_rate as usize;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(sample_size);
    loop {
        if let Ok(iter) = consumer.pop() {
            let mut buffer : Vec<_> = iter.take(sample_size).map(|v| Complex{im: 0f32, re: v}).collect();
            fft.process(&mut buffer);
            let freq : Vec<_> = buffer.iter()
                .map(|r| r.re / (sample_size as f32).sqrt())
                .collect();

            // let amp : f32 = freq[].iter().sum();
            // if amp > 0.1 {
            //     println!("{amp:#?}");
            // }
            for (n,f) in freq.iter().enumerate().skip(40).take(80) {
                println!("{n} Hz: {f:.2}\t");
            }
        };
    }
}
