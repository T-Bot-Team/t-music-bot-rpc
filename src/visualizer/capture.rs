use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::mpsc::SyncSender;
use crate::info;

pub fn start_capture(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    audio_tx: SyncSender<Vec<f32>>,
) -> cpal::Stream {
    let err_fn = |err| {
        info!("[Visualizer] Stream Error: {}", err);
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                config,
                {
                    let tx = audio_tx.clone();
                    move |data: &[f32], _| {
                        let _ = tx.try_send(data.to_vec());
                    }
                },
                err_fn,
                None,
            )
            .unwrap(),
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                config,
                {
                    let tx = audio_tx.clone();
                    move |data: &[i16], _| {
                        let vec: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        let _ = tx.try_send(vec);
                    }
                },
                err_fn,
                None,
            )
            .unwrap(),
        cpal::SampleFormat::U16 => device
            .build_input_stream(
                config,
                {
                    let tx = audio_tx.clone();
                    move |data: &[u16], _| {
                        let vec: Vec<f32> = data
                            .iter()
                            .map(|&s| (s as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0))
                            .collect();
                        let _ = tx.try_send(vec);
                    }
                },
                err_fn,
                None,
            )
            .unwrap(),
        _ => device
            .build_input_stream(
                config,
                {
                    let tx = audio_tx.clone();
                    move |data: &[f32], _| {
                        let _ = tx.try_send(data.to_vec());
                    }
                },
                err_fn,
                None,
            )
            .unwrap(),
    };

    stream.play().unwrap();
    stream
}
