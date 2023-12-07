#![allow(clippy::precedence)]

use bevy_fundsp_mic::MicConfig;
use {bevy::prelude::*, bevy_fundsp::prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DspPlugin::default())
        .add_dsp_source(
            MicConfig {
                channels: 1,
                sample_rate: default_sample_rate(),
            },
            SourceType::Dynamic,
        )
        .add_systems(PostStartup, play_noise)
        .run();
}

fn play_noise(
    mut commands: Commands,
    mut assets: ResMut<Assets<DspSource>>,
    dsp_manager: Res<DspManager>,
) {
    let source = assets.add(
        dsp_manager
            .get_graph(MicConfig {
                channels: 1,
                sample_rate: default_sample_rate(),
            })
            .unwrap_or_else(|| panic!("DSP source not found!")),
    );
    commands.spawn(AudioSourceBundle {
        source,
        ..default()
    });
}

fn default_sample_rate() -> u32 {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .unwrap_or_else(|| panic!("No output device available."));
    let default_config = device
        .default_output_config()
        .unwrap_or_else(|err| panic!("Cannot find default stream config. Error: {err}"));

    #[allow(clippy::cast_precision_loss)]
    {
        default_config.sample_rate().0
    }
}
