use crate::errors::AppError;
use crate::models::{get_models_dir, WHISPER_MODEL_FILENAME};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use rubato::{FftFixedIn, Resampler};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

const WHISPER_SAMPLE_RATE: u32 = 16000;
const MAX_DURATION_SEC: u64 = 30;

// --- COMMANDS ---
enum AudioCommand {
    Start(mpsc::Sender<Result<(), AppError>>), // Reply channel for start status
    Stop(mpsc::Sender<Result<String, AppError>>), // Reply channel for transcript
    Cancel,
}

// --- GLOBAL ACTOR ---
static AUDIO_TX: OnceLock<Mutex<mpsc::Sender<AudioCommand>>> = OnceLock::new();

pub fn init_whisper(app_handle: AppHandle) -> Result<(), AppError> {
    if AUDIO_TX.get().is_some() {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel::<AudioCommand>();
    
    // Spawn the dedicated Audio Thread
    thread::spawn(move || {
        let mut stream_handle: Option<cpal::Stream> = None;
        let mut audio_buffer: Vec<f32> = Vec::new();
        let mut device_sample_rate = 0u32;
        let mut recording_start: Option<Instant> = None;
        
        // This thread owns the Whisper Context (lazy loaded)
        let mut whisper_ctx: Option<WhisperContext> = None;
        
        // Shared buffer for the CPAL callback to write into
        let shared_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

        while let Ok(cmd) = rx.recv() {
            match cmd {
                AudioCommand::Start(reply_tx) => {
                    // Cleanup previous state
                    stream_handle = None;
                    audio_buffer.clear();
                    if let Ok(mut b) = shared_buffer.lock() { b.clear(); }

                    // WRAP SETUP IN A CLOSURE TO CATCH ERRORS EASILY
                    let setup_result = (|| -> Result<cpal::Stream, AppError> {
                        let host = cpal::default_host();
                        
                        // 1. Check Device
                        let device = host.default_input_device()
                            .ok_or_else(|| AppError::AudioDeviceNotFound("No input device found".into()))?;

                        // 2. Check Config (This is where your specific error happens)
                        let config = device.default_input_config()
                            .map_err(|e| AppError::AudioDeviceNotFound(format!("Failed to get input config: {}", e)))?;

                        device_sample_rate = config.sample_rate().0;
                        recording_start = Some(Instant::now());

                        let writer = shared_buffer.clone();
                        let evt_handle = app_handle.clone(); 
                        
                        let err_fn = |err| eprintln!("Stream error: {}", err);
                        
                        // 3. Build Stream
                        let stream = match config.sample_format() {
                            SampleFormat::F32 => device.build_input_stream(
                                &config.into(),
                                move |data: &[f32], _| {
                                    if let Some(start) = recording_start {
                                        if start.elapsed().as_secs() >= MAX_DURATION_SEC {
                                            return;
                                        }
                                    }

                                    let rms = (data.iter().map(|x| x * x).sum::<f32>() / data.len() as f32).sqrt();
                                    let _ = evt_handle.emit("audio-level", rms);
                                    if let Ok(mut b) = writer.lock() { b.extend_from_slice(data); }
                                },
                                err_fn, None
                            ),
                            SampleFormat::I16 => device.build_input_stream(
                                &config.into(),
                                move |data: &[i16], _| {
                                    if let Some(start) = recording_start {
                                        if start.elapsed().as_secs() >= MAX_DURATION_SEC {
                                            return;
                                        }
                                    }
                                    
                                    let sum_squares: f32 = data.iter().map(|&s| {
                                        let f = s as f32 / 32768.0;
                                        f * f
                                    }).sum();
                                    let rms = (sum_squares / data.len() as f32).sqrt();
                                    let _ = evt_handle.emit("audio-level", rms);

                                    if let Ok(mut b) = writer.lock() { 
                                        b.extend(data.iter().map(|&s| s as f32 / 32768.0));
                                    }
                                },
                                err_fn, None
                            ),
                            SampleFormat::U16 => device.build_input_stream(
                                &config.into(),
                                move |data: &[u16], _| {
                                    if let Some(start) = recording_start {
                                        if start.elapsed().as_secs() >= MAX_DURATION_SEC {
                                            return;
                                        }
                                    }
                                    
                                    let sum_squares: f32 = data.iter().map(|&s| {
                                        let f = (s as f32 / 32768.0) - 1.0;
                                        f * f
                                    }).sum();
                                    let rms = (sum_squares / data.len() as f32).sqrt();
                                    let _ = evt_handle.emit("audio-level", rms);

                                    if let Ok(mut b) = writer.lock() { 
                                        b.extend(data.iter().map(|&s| (s as f32 / 32768.0) - 1.0));
                                    }
                                },
                                err_fn, None
                            ),
                            _ => return Err(AppError::AudioDeviceNotFound("Unsupported sample format".into())),
                        }.map_err(|e| AppError::AudioDeviceNotFound(e.to_string()))?;

                        // 4. Play Stream
                        stream.play().map_err(|e| AppError::AudioDeviceNotFound(e.to_string()))?;
                        
                        Ok(stream)
                    })();

                    // HANDLE THE RESULT
                    match setup_result {
                        Ok(stream) => {
                            stream_handle = Some(stream);
                            let _ = reply_tx.send(Ok(())); // Tell frontend: "Success!"
                        }
                        Err(e) => {
                            let _ = reply_tx.send(Err(e)); // Tell frontend: "Failed: No Mic!"
                        }
                    }
                }
                AudioCommand::Stop(reply_tx) => {
                    // 1. Drop stream to stop recording
                    stream_handle = None; 
                    recording_start = None;

                    // 2. Retrieve data
                    let raw_samples = if let Ok(mut b) = shared_buffer.lock() {
                        std::mem::take(&mut *b)
                    } else {
                        Vec::new()
                    };

                    // 3. Process & Transcribe
                    let result = process_audio(&raw_samples, device_sample_rate, &mut whisper_ctx);
                    let _ = reply_tx.send(result);
                }
                AudioCommand::Cancel => {
                    stream_handle = None;
                    recording_start = None;
                    if let Ok(mut b) = shared_buffer.lock() { b.clear(); }
                }
            }
        }
    });

    AUDIO_TX.set(Mutex::new(tx)).map_err(|_| AppError::Unknown("Failed to set global audio channel".into()))?;
    Ok(())
}

// --- PUBLIC API ---

/// Verify model integrity by attempting to load it
pub fn verify_model_integrity(model_path: &std::path::Path) -> Result<bool, Box<dyn std::error::Error>> {
    if !model_path.exists() {
        return Ok(false);
    }
    
    // Try to load the model - if this succeeds, it's valid
    match WhisperContext::new_with_params(&model_path.to_string_lossy(), Default::default()) {
        Ok(_ctx) => {
            println!("YAAD_LOG: Model integrity verified successfully.");
            Ok(true)
        }
        Err(e) => {
            println!("YAAD_LOG: Model integrity check failed: {}", e);
            Ok(false)
        }
    }
}

pub fn start_recording() -> Result<(), AppError> {
    let (reply_tx, reply_rx) = mpsc::channel();
    let tx = AUDIO_TX.get().ok_or(AppError::Unknown("Audio system not started".into()))?.lock().unwrap();
    tx.send(AudioCommand::Start(reply_tx)).map_err(|_| AppError::AudioBusy("Audio thread dead".into()))?;
    
    // Wait for the background thread to confirm start
    reply_rx.recv().map_err(|_| AppError::Unknown("Audio thread crashed".into()))??;
    
    Ok(())
}

pub fn stop_recording() -> Result<String, AppError> {
    let (reply_tx, reply_rx) = mpsc::channel();
    let tx = AUDIO_TX.get().ok_or(AppError::Unknown("Audio system not started".into()))?.lock().unwrap();
    
    tx.send(AudioCommand::Stop(reply_tx)).map_err(|_| AppError::AudioBusy("Audio thread dead".into()))?;
    
    // Wait for the background thread to finish transcription
    reply_rx.recv().map_err(|_| AppError::Unknown("Audio thread crashed".into()))?
}

pub fn cancel_recording() -> Result<(), AppError> {
    let tx = AUDIO_TX.get().ok_or(AppError::Unknown("Audio system not started".into()))?.lock().unwrap();
    tx.send(AudioCommand::Cancel).map_err(|_| AppError::AudioBusy("Audio thread dead".into()))?;
    Ok(())
}

pub fn reset_recording_state() -> Result<(), AppError> {
    cancel_recording()
}

// --- INTERNAL HELPERS ---

fn process_audio(samples: &[f32], sample_rate: u32, ctx_cache: &mut Option<WhisperContext>) -> Result<String, AppError> {
    if samples.is_empty() { return Err(AppError::AudioTooShort); }

    // RMS Check
    let rms: f32 = (samples.iter().map(|s| s*s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms < 0.001 { return Err(AppError::AudioTooQuiet); }

    // Resample
    let resampled = if sample_rate != WHISPER_SAMPLE_RATE {
        resample_audio(samples, sample_rate, WHISPER_SAMPLE_RATE)?
    } else {
        samples.to_vec()
    };

    // Load Model if missing
    if ctx_cache.is_none() {
        let models_dir = get_models_dir().map_err(|e| AppError::FileSystemError(e.to_string()))?;
        let model_path = models_dir.join(WHISPER_MODEL_FILENAME);
        if !model_path.exists() { return Err(AppError::ModelNotFound); }
        
        let ctx = WhisperContext::new_with_params(&model_path.to_string_lossy(), Default::default())
            .map_err(|_| AppError::ModelCorrupt)?;
        *ctx_cache = Some(ctx);
    }

    // Transcribe
    let ctx = ctx_cache.as_ref().unwrap();
    let mut state = ctx.create_state().map_err(|e| AppError::TranscriptionFailed(e.to_string()))?;
    
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_token_timestamps(false);
    params.set_translate(false);
    
    // Whisper expects samples in range [-1.0, 1.0]
    let samples_f32: Vec<f32> = resampled
        .iter()
        .map(|&sample| sample.max(-1.0).min(1.0))
        .collect();

    state.full(params, &samples_f32).map_err(|e| AppError::TranscriptionFailed(e.to_string()))?;
    
    let num_segments = state.full_n_segments().unwrap_or(0);
    let mut text = String::new();
    for i in 0..num_segments {
        if let Ok(segment) = state.full_get_segment_text(i) {
             let clean_segment = segment.replace("[BLANK_AUDIO]", "").trim().to_string();
             if !clean_segment.is_empty() {
                text.push_str(&clean_segment);
                text.push(' ');
             }
        }
    }
    
    let final_result = text.trim().to_string();
    if final_result.is_empty() {
        return Err(AppError::TranscriptionFailed("No speech detected".to_string()));
    }
    
    Ok(final_result)
}

fn resample_audio(input: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>, AppError> {
    // Create resampler
    // New signature: sample_rate_in, sample_rate_out, chunk_size, sub_chunks, channels
    let mut resampler = FftFixedIn::<f32>::new(
        from_rate as usize, 
        to_rate as usize, 
        1024, 
        2, 
        1
    ).map_err(|e| AppError::Unknown(format!("Failed to create resampler: {}", e)))?;
    
    let input_chunk_size = 1024;
    let mut resampled = Vec::new();
    
    // Pre-allocate buffer for efficiency
    let mut input_buffer = vec![vec![0.0f32; input_chunk_size]];
    
    for chunk in input.chunks(input_chunk_size) {
        let chunk_len = chunk.len();
        
        if chunk_len == input_chunk_size {
            // Case A: Perfect full chunk
            input_buffer[0].copy_from_slice(chunk);
            match resampler.process(&input_buffer, None) {
                Ok(output) => {
                    if !output.is_empty() {
                        resampled.extend_from_slice(&output[0]);
                    }
                }
                Err(e) => return Err(AppError::Unknown(format!("Resampling error: {}", e))),
            }
        } else {
             // Case B: Partial last chunk
             if chunk_len >= 64 {
                let mut padded_chunk = chunk.to_vec();
                padded_chunk.resize(input_chunk_size, 0.0);
                
                input_buffer[0].copy_from_slice(&padded_chunk);
                 match resampler.process(&input_buffer, None) {
                    Ok(output) => {
                        if !output.is_empty() {
                            resampled.extend_from_slice(&output[0]);
                        }
                    }
                    Err(e) => eprintln!("Warning: Failed to resample final partial chunk: {}", e),
                }
             }
        }
    }
    
    Ok(resampled)
}
