use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use bevy::utils::synccell::SyncCell;
use bevy::utils::tracing::error;
use bevy::utils::Duration;
use bevy_fundsp::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{default_host, ChannelCount, SampleFormat, SampleRate};
use numeric_array::{ArrayLength, NumericArray};
use uuid::Uuid;

#[derive(Clone)]
pub struct MicConfig {
    pub channels: ChannelCount,
    pub sample_rate: u32,
}

impl DspGraph for MicConfig {
    fn id(&self) -> uuid::Uuid {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            std::any::type_name::<MicConfig>().as_bytes(),
        )
    }

    fn generate_graph(&self) -> Box<dyn AudioUnit32> {
        match self.channels {
            1 => Box::new(cpal_process::<U1>(self.clone())),
            2 => Box::new(cpal_process::<U2>(self.clone())),
            _ => panic!("unsupported channel count"),
        }
    }
}

fn cpal_process<N: ArrayLength<f32>>(mic_config: MicConfig) -> An<MicNode<N>> {
    let (tx, rx) = channel::<Frame<f32, N>>();
    let rx = Arc::new(Mutex::new(SyncCell::new(rx)));
    let channels = mic_config.channels;
    std::thread::spawn(move || {
        let device = default_host().default_input_device().unwrap();
        let mut configs = device.supported_input_configs().unwrap();
        let config = configs
            .find(|c| {
                c.sample_format() == SampleFormat::F32
                    && c.channels() == channels
                    && c.min_sample_rate().0 < mic_config.sample_rate
                    && c.max_sample_rate().0 > mic_config.sample_rate
            })
            .unwrap()
            .with_sample_rate(SampleRate(mic_config.sample_rate));
        let err_fn = |err| error!("an error occurred on the output audio stream: {}", err);
        let stream = device
            .build_input_stream(
                &config.into(),
                move |d: &[f32], _| {
                    println!("{:?}", d);
                    for slice in d.chunks(channels as _) {
                        tx.send(NumericArray::from_slice(slice).clone()).unwrap();
                    }
                },
                err_fn,
                None,
            )
            .unwrap();
        stream.play().unwrap();
        loop {
            std::thread::sleep(Duration::from_secs(100));
        }
    });
    An(MicNode { rx })
}

#[derive(Clone)]
struct MicNode<N: ArrayLength<f32>> {
    rx: Arc<Mutex<SyncCell<Receiver<Frame<f32, N>>>>>,
}

impl<N: Size<f32>> AudioNode for MicNode<N> {
    const ID: u64 = 1446762434551402895;

    type Sample = f32;

    type Inputs = numeric_array::typenum::U0;

    type Outputs = N;

    type Setting = ();

    fn tick(
        &mut self,
        _: &Frame<Self::Sample, Self::Inputs>,
    ) -> Frame<Self::Sample, Self::Outputs> {
        // self.rx.get().recv().unwrap_or_default()
        let mut rx = self.rx.lock().unwrap();
        rx.get().recv().unwrap_or_default()
    }

    fn process(&mut self, size: usize, _: &[&[Self::Sample]], output: &mut [&mut [Self::Sample]]) {
        let mut guard = self.rx.lock().unwrap();
        let rx = guard.get();
        for i in 0..size {
            let result = rx.recv().unwrap_or_default();
            for (x, y) in output.iter_mut().zip(result.iter()) {
                (*x)[i] = *y;
            }
        }
    }
}
