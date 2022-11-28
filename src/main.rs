use structopt::StructOpt;

use iced::{
    executor, keyboard, time,
    widget::{button, container::Container, text, Column},
    window, Alignment, Application, Command, Element, Settings, Subscription, Theme,
};
use iced_native::subscription;

use std::time::Duration;

use cpal::traits::DeviceTrait;

mod sound_proxy;
use sound_proxy::SoundProxy;

mod sound_transformer;

mod spectrum_visualization;
use spectrum_visualization::{Visualizer, VisualizerMessage};

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
pub enum ContentType {
    Raw,
    Processed,
}

#[derive(Default, Clone)]
pub struct Sides<T> {
    left: T,
    right: T,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
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
    state: AppState,
    sound_proxy: SoundProxy,
    visualizer: Visualizer,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = AppMessage;
    type Flags = Opt;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                debug: flags.debug,

                should_exit: false,
                state: AppState::SelectingSource,
                visualizer: Visualizer::new(
                    flags.width,
                    flags.height,
                    ContentType::Processed,
                    DisplayType::Lines,
                    Sides::<Vec<f32>>::default(),
                    true,
                ),
                sound_proxy: SoundProxy::default(),
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
                    } => Some(AppMessage::Quit),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Z,
                        ..
                    } => Some(AppMessage::ScanDevices),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::M,
                        ..
                    } => Some(AppMessage::UnselectDevice),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::P,
                        ..
                    } => Some(AppMessage::SwitchDisplayContent),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::N,
                        ..
                    } => Some(AppMessage::ToggleNormalize),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::S,
                        ..
                    } => Some(AppMessage::ToggleSmooth),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::F,
                        ..
                    } => Some(AppMessage::ToggleFlashFlood),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Period, // >
                        ..
                    } => Some(AppMessage::ShiftMovingAvgRange(1)),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Comma, // <
                        ..
                    } => Some(AppMessage::ShiftMovingAvgRange(-1)),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::O,
                        ..
                    } => Some(AppMessage::ToggleOffCenter),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Up,
                        ..
                    } => Some(AppMessage::ScaleUp),

                    keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Down,
                        ..
                    } => Some(AppMessage::ScaleDown),

                    _ => None,
                }
            }
            _ => None,
        });

        let ticks = if let AppState::Displaying = self.state {
            time::every(Duration::from_millis(10)).map(|_| AppMessage::Tick)
        } else {
            Subscription::none()
        };

        Subscription::batch(vec![events, ticks])
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        if self.debug {
            if let AppMessage::Tick = message {
                // don't print for ticks, that would clog the console
            } else {
                println!("Message: {:?}", message)
            }
        }

        match message {
            AppMessage::Quit => {
                self.should_exit = true;
            }
            AppMessage::ScanDevices => {
                self.sound_proxy.scan_devices();
            }
            AppMessage::SelectDevice(index) => {
                self.state = AppState::Displaying;
                self.sound_proxy.select_device(index);
            }
            AppMessage::UnselectDevice => {
                self.state = AppState::SelectingSource;
                self.sound_proxy.unselect_device();
            }

            // pass through to the visualizer
            AppMessage::SwitchDisplayContent => self
                .visualizer
                .update(VisualizerMessage::SwitchDisplayContent),
            AppMessage::ToggleNormalize => {
                self.visualizer.update(VisualizerMessage::ToggleNormalize)
            }
            AppMessage::ToggleSmooth => self.visualizer.update(VisualizerMessage::ToggleSmooth),
            AppMessage::ToggleFlashFlood => {
                self.visualizer.update(VisualizerMessage::ToggleFlashFlood)
            }
            AppMessage::ShiftMovingAvgRange(val) => self
                .visualizer
                .update(VisualizerMessage::ShiftMovingAvgRange(val)),
            AppMessage::ScaleUp => self.visualizer.update(VisualizerMessage::ScaleUp),
            AppMessage::ScaleDown => self.visualizer.update(VisualizerMessage::ScaleDown),
            AppMessage::ToggleOffCenter => {
                self.visualizer.update(VisualizerMessage::ToggleOffCenter)
            }
            AppMessage::Tick => {
                if let AppState::Displaying = self.state {
                    self.visualizer
                        .update(VisualizerMessage::UpdateContent(Box::new(
                            self.sound_proxy.get_clip(),
                        )));
                }
            }
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
                            button(text(device.name().expect("device name")))
                                .on_press(AppMessage::SelectDevice(i)),
                        )
                    },
                );

                Container::new(buttons).into()
            }
            AppState::Displaying => self.visualizer.view(),
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
