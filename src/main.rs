
use hueclient::{CommandLight, Bridge};

use rodio::Sink;
use rodio::{source::Source, Decoder, OutputStream};

use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, sleep};
use std::{fs::File, time::Duration};

fn lights(receiver: Receiver<()>) {
    let bridge =
        hueclient::Bridge::discover_required().with_user(std::env::var("HUE_USER").unwrap());

    let lights = bridge.get_all_lights().unwrap();
    let n = lights.len();


    let mut i = 0;
    const DELAY : u64 = 200;
    loop {
        receiver.recv().unwrap();
        sleep(Duration::from_millis(DELAY));
        let light = &lights[i];
        fire(&bridge, light.id);
        i = (i + 1) % n;
    }
}

fn fire(bridge: &Bridge, light: usize) {
    let impact = CommandLight::default().with_bri(255)
        .with_hue(41000)
        .with_sat(70)
        .with_xy(0.2, 0.2);
    let normal = CommandLight::default().with_bri(70)
        .with_sat(200)
        .with_hue(45555)
        .with_xy(0.8, 0.8);
    bridge.set_light_state(light, &impact).unwrap_or_else(|e| {
        eprintln!("{e}");
        ().into()
    });
    sleep(Duration::from_millis(150));
    bridge.set_light_state(light, &normal).unwrap_or_else(|e| {
        eprintln!("{e}");
        ().into()
    });
}

fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let file = BufReader::new(File::open("rain.mp3").unwrap());
    let source = Decoder::new(file).unwrap();

    // let source = SineWave::new(50.0);

    let sample_rate = source.sample_rate();
    let (producer, consumer) = mpsc::channel();
    let source =
        source
            .convert_samples()
            .buffered()
            .periodic_access(Duration::from_millis(100), move |s| {
                let s = s.clone();
                producer.send(s);
                // let s : u32 = s.fold(0, |acc : u32, x : i16| acc.saturating_add(x.unsigned_abs() as u32));
                // println!("{s}")
                // producer.push(*s);
            });

    let sink = Sink::try_new(&stream_handle).unwrap();

    sink.append(source);

    // Spawn a thread periodically run FFT on the sound stream
    std::thread::spawn(move || {
        fft_check(consumer, sample_rate);
    });
    sink.sleep_until_end();
}

fn crude_amp_check<I: Iterator<Item = f32>>(consumer: Receiver<I>) -> ! {
    loop {
        if let Ok(iter) = consumer.recv() {
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

fn fft_check<I: Source<Item = f32>>(consumer: Receiver<I>, sample_rate: u32) -> ! {
    const RESOLUTION: usize = 50;
    let sample_size = sample_rate as usize / RESOLUTION;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(sample_size);

    let (light, light_rec) = mpsc::channel();
    thread::spawn(move || lights(light_rec));
    loop {
        if let Ok(iter) = consumer.recv() {
            // let t0 = std::time::Instant::now();
            let mut buffer: Vec<_> = iter
                .take(sample_size)
                .map(|v| Complex { im: 0f32, re: v })
                .collect();
            fft.process(&mut buffer);
            let freq: Vec<_> = buffer
                .iter()
                .map(|c| c.re / (sample_size as f32).sqrt())
                .map(|r| r.abs())
                // .map(|r| r.re)
                .collect();

            // let amp : f32 = freq[0..=1].iter().sum();
            // if amp > 1.0 {
            //     println!("{amp:#?}");
            // }
            // let t1 = std::time::Instant::now();
            // println!("{:#?}", t1-t0);

            // for (n, f) in freq.iter().enumerate().skip(0).take(10) {
            //     let n = n * RESOLUTION;
            //     if *f > 1.00 {
            //         print!("{n} Hz: {f:.2}\t");
            //     } else {
            //         print!("{n} Hz:     \t");
            //     }
            // }
            let power = freq[1..=4].iter().sum::<f32>();
            if power > 3.0 {
                println!("{power}");
                light.send(()).unwrap();
            }
        };
    }
}
