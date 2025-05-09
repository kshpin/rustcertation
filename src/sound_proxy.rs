use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleRate, Stream, StreamConfig, SupportedStreamConfigRange};

use ringbuffer::{ConstGenericRingBuffer, RingBufferExt};

const CLIP_CAP: usize = 4096;

#[derive(Clone)]
pub struct Clip {
    pub sample_rate: u32,

    pub left: ConstGenericRingBuffer<f32, CLIP_CAP>,
    pub right: ConstGenericRingBuffer<f32, CLIP_CAP>,
}

impl Default for Clip {
    fn default() -> Clip {
        let mut left = ConstGenericRingBuffer::<f32, CLIP_CAP>::new();
        left.fill_default();
        let mut right = ConstGenericRingBuffer::<f32, CLIP_CAP>::new();
        right.fill_default();

        Self {
            sample_rate: 0,

            left,
            right,
        }
    }
}

unsafe impl Send for Clip {}
unsafe impl Sync for Clip {}

// custom de-interleaving iterator
struct RawSoundData<'a> {
    data: &'a [f32],
    num_channels: usize,
    pos: usize,
}

impl<'a> Iterator for RawSoundData<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            None
        } else {
            let val = self.data[self.pos];
            self.pos += self.num_channels;
            Some(val)
        }
    }
}

pub struct SoundProxy {
    _sound_host: Host,
    devices: Vec<Device>,

    clip: Arc<Mutex<Clip>>,
    stream: Option<Stream>,
}

impl Default for SoundProxy {
    fn default() -> SoundProxy {
        let sound_host = cpal::default_host();
        let devices = scan_devices(&sound_host);

        Self {
            _sound_host: sound_host,
            devices,

            clip: Arc::new(Mutex::new(Clip::default())),
            stream: None,
        }
    }
}

// public
impl SoundProxy {
    pub fn scan_devices(&mut self) {
        self.devices = scan_devices(&self._sound_host);
    }

    pub fn get_devices(&self) -> &Vec<Device> {
        &self.devices
    }

    pub fn get_clip(&self) -> Clip {
        self.clip
            .clone()
            .lock()
            .expect("locked Clip in get_clip")
            .clone()
    }

    pub fn select_device(&mut self, index: usize) {
        let device = &self.devices[index];

        let device_name = device.name().expect("device name in select_device");

        let mut usable_configs: Vec<SupportedStreamConfigRange> = device
            .supported_input_configs()
            .expect("device's supported configs")
            /* .map(|config| {
                println!("{:#?}", config);
                config
            }) */
            .filter(|config| config.channels() <= 2)
            .collect();
        usable_configs.sort_unstable_by_key(|config| -(config.channels() as i16));

        let config: StreamConfig = usable_configs
            .into_iter()
            .next()
            .expect("config to use in select_device")
            //.with_max_sample_rate()
            .with_sample_rate(SampleRate(44100))
            .into();

        println!("[{}]'s config: {:#?}", device_name, config);

        let clip_clone = self.clip.clone();
        let mut locked_clip = self
            .clip
            .lock()
            .expect("locked Clip mutex in select_device");

        locked_clip.sample_rate = config.sample_rate.0;

        let stream = device
            .build_input_stream(
                &config,
                move |data, _| {
                    on_data(
                        &mut clip_clone
                            .lock()
                            .expect("locked Clip mutex in data_callback"),
                        data,
                    )
                },
                |error| eprintln!("{}", error),
            )
            .expect("stream in select_device");

        // have to play the stream
        stream.play().expect("playing stream in select_device");
        self.stream = Some(stream);
    }

    pub fn unselect_device(&mut self) {
        self.stream = None;
    }
}

fn on_data(clip: &mut Clip, data: &[f32]) {
    clip.left.extend(RawSoundData {
        data,
        num_channels: 2,
        pos: 0,
    });
    clip.right.extend(RawSoundData {
        data,
        num_channels: 2,
        pos: 1,
    });
}

// function instead of method so that it can be reused in the constructor
fn scan_devices(sound_host: &Host) -> Vec<Device> {
    sound_host
        .devices()
        .expect("iterator of available devices")
        //.into_iter()
        .filter(|device| {
            let possibly_supported_configs = device.supported_input_configs();

            if let Ok(mut supported_configs) = possibly_supported_configs {
                supported_configs.any(|config| config.channels() <= 2)
            } else {
                false
            }
        }) // keep only input devices
        .collect()
    //vec![sound_host.default_input_device().expect("default input device")]
}
