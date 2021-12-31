use structopt::StructOpt;

use iced::{
    button, canvas, executor, keyboard, time, Align, Application, Button, Color, Column, Command,
    Container, Element, Length, Settings, Subscription, Text,
};
use iced_native::subscription;

use std::time::Duration;

use cpal::traits::DeviceTrait;

use ringbuffer::RingBufferExt;

use spectrum_analyzer::{
    self, samples_fft_to_spectrum, windows::hann_window, FrequencyLimit, FrequencySpectrum,
};

mod sound_proxy;
use sound_proxy::SoundProxy;

mod spectrum_visualization;
use spectrum_visualization::SpectrumViz;

enum AppState {
    SelectingSource,
    Displaying,
}

#[derive(Clone, Copy)]
pub enum DisplayType {
    Lines,
    Boxes,
    Circle,
}

#[derive(Clone, Copy)]
pub enum DisplayContent {
    Raw,
    Processed,
}

pub struct Sides<T> {
    left: T,
    right: T,
}

struct SoundData {
    raw: Sides<Vec<f32>>,
    freqs: Sides<Vec<f32>>,
}

/* struct SelectMenu<T> {
    options: Vec<(T, button::State)>,
} */

#[derive(Debug, Clone)]
enum Message {
    Quit,
    ScanDevices,
    SelectDevice(usize),
    UnselectDevice,
    SwitchDisplayContent,
    Tick,
}

struct App {
    should_exit: bool,

    width: u32,
    height: u32,

    clear_color: Color,

    button_states: Vec<button::State>,

    state: AppState,
    display_content: DisplayContent,
    display_type: DisplayType,

    sound_proxy: SoundProxy,
    sound_data: Option<SoundData>,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = Opt;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                should_exit: false,

                width: flags.width,
                height: flags.height,

                clear_color: Color::from_rgb8(51, 51, 51), // #333333

                button_states: Vec::new(),

                state: AppState::SelectingSource,
                display_content: DisplayContent::Processed,
                display_type: DisplayType::Lines,

                sound_proxy: SoundProxy::default(),
                sound_data: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rustcertation (on the rocks)")
    }

    fn background_color(&self) -> Color {
        self.clear_color
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let events = subscription::events_with(|event, _status| match event {
            iced_native::Event::Keyboard(keyboard_event) => match keyboard_event {
                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::W,
                    modifiers: keyboard::Modifiers { control: true, .. },
                } => Some(Message::Quit),

                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::S,
                    ..
                } => Some(Message::ScanDevices),

                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::M,
                    ..
                } => Some(Message::UnselectDevice),

                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::P,
                    ..
                } => Some(Message::SwitchDisplayContent),

                _ => None,
            },
            _ => None,
        });

        let ticks = if let AppState::Displaying = self.state {
            time::every(Duration::from_millis(10)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };

        Subscription::batch(vec![events, ticks])
    }

    fn update(
        &mut self,
        message: Self::Message,
        _clipboard: &mut iced::Clipboard,
    ) -> iced::Command<Self::Message> {
        match message {
            Message::Quit => {
                self.should_exit = true;
            }
            Message::ScanDevices => {
                self.sound_proxy.scan_devices();
            }
            Message::SelectDevice(index) => {
                self.state = AppState::Displaying;
                self.sound_proxy.select_device(index);
            }
            Message::UnselectDevice => {
                self.state = AppState::SelectingSource;
                self.sound_proxy.unselect_device();
            }
            Message::SwitchDisplayContent => {
                self.display_content = match self.display_content {
                    DisplayContent::Raw => {
                        println!("showing frequencies");
                        DisplayContent::Processed
                    }
                    DisplayContent::Processed => {
                        println!("showing raw sound");
                        DisplayContent::Raw
                    }
                };
            }
            Message::Tick => {
                match self.state {
                    AppState::SelectingSource => {
                        // don't have to do anything at all
                    }
                    AppState::Displaying => {
                        let clip_mutex = self.sound_proxy.get_clip();
                        let clip = clip_mutex.lock().expect("locked Clip in update");

                        let raw = Sides {
                            left: clip.left.to_vec(),
                            right: clip.right.to_vec(),
                        };

                        let freqs = Sides {
                            left: get_freqs(&raw.left, clip.sample_rate)
                                .data()
                                .iter()
                                .map(|(_, v)| v.val())
                                .collect(),
                            right: get_freqs(&raw.right, clip.sample_rate)
                                .data()
                                .iter()
                                .map(|(_, v)| v.val())
                                .collect(),
                        };

                        self.sound_data = Some(SoundData { raw, freqs });
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        self.button_states = Vec::new();

        match self.state {
            AppState::SelectingSource => {
                let devices = self.sound_proxy.get_devices();
                for _ in 0..devices.len() {
                    self.button_states.push(button::State::default());
                }

                let buttons = self
                    .button_states
                    .iter_mut()
                    .zip(devices.iter())
                    .enumerate()
                    .fold(
                        Column::new().align_items(Align::Start),
                        |column, (i, (state, device))| {
                            column.push(
                                Button::new(state, Text::new(device.name().expect("device name")))
                                    .on_press(Message::SelectDevice(i)),
                            )
                        },
                    );

                Container::new(buttons).into()
            }
            AppState::Displaying => {
                // draw fourier transforms and such

                if let Some(SoundData { raw, freqs }) = &self.sound_data {
                    let to_draw = if let DisplayContent::Raw = self.display_content {
                        raw
                    } else {
                        freqs
                    };

                    Container::new(
                        canvas::Canvas::new(SpectrumViz::new(
                            self.display_content,
                            self.display_type,
                            to_draw,
                        ))
                        .width(Length::Units(self.width as u16))
                        .height(Length::Units(self.height as u16)),
                    )
                    .into() // seems like the canvas constructor expects something that accepts the same messages
                } else {
                    Container::new(Text::new("nothing to draw :/")).into()
                }
            }
        }
    }
}

fn get_freqs(data: &[f32], sample_rate: u32) -> FrequencySpectrum {
    samples_fft_to_spectrum(&hann_window(data), sample_rate, FrequencyLimit::All, None)
        .expect("frequency spectrum conversion")
}

#[derive(StructOpt, Debug)]
struct Opt {
    /// Run in debug mode
    #[structopt(short = "d")]
    debug: bool,

    /// Set window width
    #[structopt(long = "width", default_value = "800")]
    width: u32,

    /// Set window height
    #[structopt(long = "height", default_value = "800")]
    height: u32,
}

fn main() -> iced::Result {
    let opt = Opt::from_args();
    if opt.debug {
        println!("options: {:?}", opt);
    }

    App::run(Settings::with_flags(opt))
}
