use structopt::StructOpt;

use iced::{
    executor, keyboard, time,
    widget::{button::Button, canvas, container::Container, text::Text, Column},
    window, Alignment, Application, Command, Element, Length, Settings, Subscription, Theme,
};
use iced_native::subscription;

use std::time::Duration;

use cpal::traits::DeviceTrait;

use ringbuffer::RingBufferExt;

use spectrum_analyzer::{self, samples_fft_to_spectrum, windows, FrequencyLimit};

mod sound_proxy;
use sound_proxy::SoundProxy;

mod sound_transformer;
use sound_transformer::SoundTransformer;

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
    ToggleNormalize,
    ToggleSmooth,
    ToggleFlashFlood,
    ShiftMovingAvgRange(i32),
    ToggleOffCenter,
    ScaleUp,
    ScaleDown,
    Tick,
}

struct App {
    debug: bool,

    should_exit: bool,

    width: u32,
    height: u32,

    state: AppState,
    display_content: DisplayContent,
    display_type: DisplayType,

    sound_proxy: SoundProxy,
    sound_data: Option<SoundData>,

    sound_transformer: SoundTransformer,

    off_center: bool,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = Opt;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                debug: flags.debug,

                should_exit: false,

                width: flags.width,
                height: flags.height,

                state: AppState::SelectingSource,
                display_content: DisplayContent::Processed,
                display_type: DisplayType::Lines,

                sound_proxy: SoundProxy::default(),
                sound_data: None,

                sound_transformer: SoundTransformer::default(),

                off_center: true,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rustcertation (on the rocks)")
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let events = subscription::events_with(|event, _status| match event {
            iced_native::Event::Keyboard(keyboard_event) => {
                match keyboard_event {
                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::W,
                        modifiers: keyboard::Modifiers::CTRL,
                    } => Some(Message::Quit),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Z,
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

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::N,
                        ..
                    } => Some(Message::ToggleNormalize),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::S,
                        ..
                    } => Some(Message::ToggleSmooth),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::F,
                        ..
                    } => Some(Message::ToggleFlashFlood),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Period, // >
                        ..
                    } => Some(Message::ShiftMovingAvgRange(1)),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Comma, // <
                        ..
                    } => Some(Message::ShiftMovingAvgRange(-1)),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::O,
                        ..
                    } => Some(Message::ToggleOffCenter),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Up,
                        ..
                    } => Some(Message::ScaleUp),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Down,
                        ..
                    } => Some(Message::ScaleDown),

                    _ => None,
                }
            }
            _ => None,
        });

        let ticks = if let AppState::Displaying = self.state {
            time::every(Duration::from_millis(10)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };

        Subscription::batch(vec![events, ticks])
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        if self.debug {
            if let Message::Tick = message {
                // don't print for ticks, that would clog the console
            } else {
                println!("Message: {:?}", message)
            }
        }

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
            Message::ToggleNormalize => self.sound_transformer.toggle_norm(),
            Message::ToggleSmooth => self.sound_transformer.toggle_smooth(),
            Message::ToggleFlashFlood => self.sound_transformer.toggle_flash_flood(),
            Message::ShiftMovingAvgRange(val) => self
                .sound_transformer
                .shift_moving_avg_range(val, self.debug),
            Message::ScaleUp => self.sound_transformer.shift_norm_scale(1.15f32),
            Message::ScaleDown => self.sound_transformer.shift_norm_scale(1f32 / 1.15f32),
            Message::ToggleOffCenter => self.off_center = !self.off_center,
            Message::Tick => match self.state {
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

                    let to_freqs = |data, sample_rate| {
                        samples_fft_to_spectrum(
                            &windows::hamming_window(data),
                            sample_rate,
                            FrequencyLimit::All,
                            None,
                        )
                        .expect("frequency spectrum conversion")
                    };

                    // define procedure ahead of time to apply to both left and right
                    let process = |new_raws, old_freqs| {
                        to_freqs(new_raws, clip.sample_rate)
                            .data()
                            .iter()
                            //.map(|(_, v)| v.val()) // keep only the important part
                            .zip(old_freqs) // use old value too for smoothing
                            //.enumerate() // normalization uses this?
                            .map(|((freq, new), old): (&(_, _), &f32)| {
                                // apply the prettifying transformation
                                self.sound_transformer.apply(*old, new.val(), freq.val())
                            })
                            .collect()
                    };

                    let freqs = if let Some(SoundData { freqs, .. }) = &self.sound_data {
                        Sides {
                            left: process(&raw.left, &freqs.left),
                            right: process(&raw.right, &freqs.right),
                        }
                    } else {
                        Sides {
                            left: vec![0f32; raw.left.len()],
                            right: vec![0f32; raw.left.len()],
                        }
                    };

                    self.sound_data = Some(SoundData { raw, freqs });
                }
            },
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        match self.state {
            AppState::SelectingSource => {
                let devices = self.sound_proxy.get_devices();

                let buttons = devices.iter().enumerate().fold(
                    Column::new().align_items(Alignment::Start),
                    |column, (i, device)| {
                        column.push(
                            Button::new(Text::new(device.name().expect("device name")))
                                .on_press(Message::SelectDevice(i)),
                        )
                    },
                );

                Container::new(buttons).into()
            }
            AppState::Displaying => {
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
                            self.off_center,
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

    App::run(Settings {
        window: window::Settings {
            size: (opt.width, opt.height),
            resizable: false,
            decorations: true,
            min_size: None,
            max_size: None,
            transparent: false,
            always_on_top: false,
            icon: None,
            position: window::Position::Centered,
            visible: true,
        },
        ..Settings::with_flags(opt)
    })
}
