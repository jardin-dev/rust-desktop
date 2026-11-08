//! Capture de la sortie audio système (loopback WASAPI) et analyse en 256 bandes.
//!
//! Équivalent Rust du plugin `AudioLevel` utilisé par le skin : FFT 8192 sur la
//! sortie par défaut, bandes réparties logarithmiquement entre 15 Hz et 24 kHz,
//! attaque immédiate et décroissance exponentielle.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, initialize_mta};

pub const BAND_COUNT: usize = 256;

const FREQ_MIN: f32 = 15.0;
const FFT_SIZE: usize = 8192;
const FFT_HOP: usize = 2048;
const FREQ_MAX: f32 = 24_000.0;
const DECAY_SECONDS: f32 = 0.150;
const DB_FLOOR: f32 = -72.0;

pub struct AudioEngine {
    bands: Arc<Mutex<Vec<f32>>>,
    stop: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn start() -> Self {
        let bands = Arc::new(Mutex::new(vec![0.0; BAND_COUNT]));
        let stop = Arc::new(AtomicBool::new(false));

        let thread_bands = Arc::clone(&bands);
        let thread_stop = Arc::clone(&stop);
        let spawned = thread::Builder::new()
            .name("miklfe-audio".into())
            .spawn(move || {
                let _ = initialize_mta();
                while !thread_stop.load(Ordering::Relaxed) {
                    if let Err(error) = capture_loop(&thread_bands, &thread_stop) {
                        eprintln!("miklfe: capture audio indisponible: {error}");
                        if let Ok(mut bands) = thread_bands.lock() {
                            bands.fill(0.0);
                        }
                        thread::sleep(Duration::from_secs(3));
                    }
                }
            });

        if let Err(error) = spawned {
            eprintln!("miklfe: impossible de démarrer le thread audio: {error}");
        }

        Self { bands, stop }
    }

    /// Copie les niveaux courants dans `out` (0.0 à 1.0).
    pub fn fill_bands(&self, out: &mut [f32]) {
        if let Ok(bands) = self.bands.lock() {
            let count = out.len().min(bands.len());
            out[..count].copy_from_slice(&bands[..count]);
            out[count..].fill(0.0);
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn capture_loop(
    shared: &Arc<Mutex<Vec<f32>>>,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let enumerator = DeviceEnumerator::new()?;
    let device = enumerator.get_default_device(&Direction::Render)?;
    let mut client = device.get_iaudioclient()?;

    let format = client.get_mixformat()?;
    let (_, min_period) = client.get_device_period()?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_period,
    };
    // Le client est créé sur un périphérique de rendu, donc une capture en mode
    // `Direction::Render` correspond au loopback système.
    client.initialize_client(&format, &Direction::Capture, &mode)?;

    let sample_rate = format.get_samplespersec();
    let channels = format.get_nchannels() as usize;
    let block_align = format.get_blockalign() as usize;
    let bits = format.get_bitspersample();
    let is_float = matches!(format.get_subformat(), Ok(SampleType::Float));
    if bits != 32 || !is_float {
        return Err("format audio non pris en charge".into());
    }

    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    client.start_stream()?;

    let mut queue: VecDeque<u8> = VecDeque::with_capacity(1 << 18);
    let mut samples: VecDeque<f32> = VecDeque::with_capacity(FFT_SIZE * 2);
    let mut new_samples = 0usize;

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
    let mut buffer = vec![Complex::new(0.0, 0.0); FFT_SIZE];

    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| {
            let x = std::f32::consts::TAU * i as f32 / (FFT_SIZE - 1) as f32;
            0.5 * (1.0 - x.cos())
        })
        .collect();

    let edges = band_edges(sample_rate as f32);
    let mut levels = vec![0.0f32; BAND_COUNT];
    let mut published = vec![0.0f32; BAND_COUNT];
    let mut last_analysis = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        if capture.read_from_device_to_deque(&mut queue).is_err() {
            break;
        }

        while queue.len() >= block_align {
            let frame: Vec<u8> = (0..block_align)
                .map(|_| queue.pop_front().unwrap())
                .collect();
            let mut mono = 0.0f32;
            for channel in 0..channels {
                let offset = channel * 4;
                mono +=
                    f32::from_le_bytes(frame[offset..offset + 4].try_into().unwrap());
            }
            mono /= channels as f32;
            samples.push_back(mono);
            new_samples += 1;
        }

        while samples.len() > FFT_SIZE {
            samples.pop_front();
        }

        if new_samples >= FFT_HOP && samples.len() >= FFT_SIZE {
            new_samples = 0;
            for (i, slot) in buffer.iter_mut().enumerate() {
                let sample = samples.get(i).copied().unwrap_or(0.0);
                *slot = Complex::new(sample * window[i], 0.0);
            }
            fft.process_with_scratch(&mut buffer, &mut scratch);

            let now = Instant::now();
            let dt = now.duration_since(last_analysis).as_secs_f32().max(0.001);
            last_analysis = now;
            let decay = (-dt / DECAY_SECONDS).exp();

            for band in 0..BAND_COUNT {
                let peak = band_peak(&buffer, edges[band], edges[band + 1]);
                let db = 20.0 * peak.max(1.0e-6).log10();
                let level = ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0);
                levels[band] = level.max(levels[band] * decay);
                published[band] = levels[band];
            }

            if let Ok(mut shared) = shared.lock() {
                shared.copy_from_slice(&published);
            }
        }

        // Sans nouvel événement on rend la main au système, mais pas trop
        // longtemps pour continuer à vider le tampon.
        if event.wait_for_event(500).is_err() {
            break;
        }
    }

    let _ = client.stop_stream();
    Ok(())
}

fn band_edges(sample_rate: f32) -> Vec<usize> {
    let bin_hz = sample_rate / FFT_SIZE as f32;
    (0..=BAND_COUNT)
        .map(|band| {
            let t = band as f32 / BAND_COUNT as f32;
            let freq = FREQ_MIN * (FREQ_MAX / FREQ_MIN).powf(t);
            ((freq / bin_hz).floor() as usize).clamp(0, FFT_SIZE / 2)
        })
        .collect()
}

fn band_peak(buffer: &[Complex<f32>], start: usize, end: usize) -> f32 {
    let end = end.max(start + 1).min(buffer.len() / 2);
    let mut peak = 0.0f32;
    for bin in start..end {
        let value = buffer[bin].norm() * 2.0 / FFT_SIZE as f32;
        if value > peak {
            peak = value;
        }
    }
    peak
}
