use std::iter;
use std::sync::{Arc, Mutex};

use iced::widget::canvas::{
    gradient::Linear, path, stroke::Style, Canvas, Cursor, Frame, Geometry, LineCap, LineDash,
    LineJoin, Program, Stroke,
};
use iced::widget::Container;
use iced::{Color, Element, Length, Rectangle, Theme};
use iced_graphics::gradient::ColorStop;
use iced_graphics::{Gradient, Point};

use palette::RgbHue;
use palette::{convert::IntoColor, Hsv, Hue, Srgb};
use ringbuffer::RingBufferExt;
use spectrum_analyzer::{samples_fft_to_spectrum, windows, FrequencyLimit};

use crate::sound_proxy::Clip;
use crate::sound_transformer::SoundTransformer;
use crate::{AppMessage, ContentType, Sides};

pub enum VisualizerMessage {
    SwitchDisplayContent,
    ToggleNormalize,
    ToggleSmooth,
    ToggleFlashFlood,
    ShiftMovingAvgRange(i32),
    ScaleUp,
    ScaleDown,
    ToggleOffCenter,
    UpdateContent(Box<Clip>),
}

pub struct Visualizer {
    width: u32,
    height: u32,

    content_type: crate::ContentType,
    display_type: crate::DisplayType,

    content: Arc<Mutex<crate::Sides<Vec<f32>>>>,

    sound_transformer: SoundTransformer,

    off_center: bool,
}

impl Visualizer {
    pub fn new(
        width: u32,
        height: u32,
        content_type: crate::ContentType,
        display_type: crate::DisplayType,
        off_center: bool,
    ) -> Self {
        Self {
            width,
            height,
            content_type,
            display_type,
            content: Arc::new(Mutex::new(Sides::<Vec<f32>>::default())),
            sound_transformer: SoundTransformer::default(),
            off_center,
        }
    }
}

impl Visualizer {
    pub fn update(&mut self, message: VisualizerMessage) {
        match message {
            VisualizerMessage::SwitchDisplayContent => {
                self.content_type = match self.content_type {
                    ContentType::Raw => {
                        println!("showing frequencies");
                        ContentType::Processed
                    }
                    ContentType::Processed => {
                        println!("showing raw sound");
                        ContentType::Raw
                    }
                };
            }
            VisualizerMessage::ToggleNormalize => self.sound_transformer.toggle_norm(),
            VisualizerMessage::ToggleSmooth => self.sound_transformer.toggle_smooth(),
            VisualizerMessage::ToggleFlashFlood => self.sound_transformer.toggle_flash_flood(),
            VisualizerMessage::ShiftMovingAvgRange(val) => {
                self.sound_transformer.shift_moving_avg_range(val, true) // TODO: make debug actually be dynamic
            }
            VisualizerMessage::ScaleUp => self.sound_transformer.shift_norm_scale(1.15f32),
            VisualizerMessage::ScaleDown => self.sound_transformer.shift_norm_scale(1f32 / 1.15f32),
            VisualizerMessage::ToggleOffCenter => self.off_center = !self.off_center,
            VisualizerMessage::UpdateContent(clip) => {
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
                let process = |new_raws, old_freqs: &Vec<f32>| {
                    to_freqs(new_raws, clip.sample_rate)
                        .data()
                        .iter()
                        //.map(|(_, v)| v.val()) // keep only the important part
                        .zip(old_freqs.iter().chain(iter::repeat(&0f32))) // use old value too for smoothing, and lengthen the iterator if needed
                        //.enumerate() // normalization uses this?
                        .map(|((freq, new), old): (&(_, _), &f32)| {
                            // apply the prettifying transformation
                            self.sound_transformer.apply(*old, new.val(), freq.val())
                        })
                        .collect()
                };

                let old_content_lock = self.content.clone();
                let mut old_content = old_content_lock
                    .lock()
                    .expect("locked content in Visualizer::update");

                let new_content = if let ContentType::Raw = self.content_type {
                    raw
                } else {
                    Sides {
                        left: process(&raw.left, &old_content.left),
                        right: process(&raw.right, &old_content.right),
                    }
                };

                *old_content = new_content;
            }
        };
    }

    pub fn view(&self) -> Element<AppMessage> {
        Container::new(
            Canvas::new(self)
                .width(Length::Units(self.width as u16))
                .height(Length::Units(self.height as u16)),
        )
        .into()
    }
}

impl Program<AppMessage> for Visualizer {
    type State = Sides<Vec<f32>>;

    fn draw(
        &self,
        _state: &Self::State,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // TODO: play around with colors more
        // start at green, which is brighter than red, then rotate back to red, which doesn't actually yield back red :/
        let red = Hsv::new(0f32, 1f32, 1f32);
        //let red = yellow.shift_hue(LabHue::from_degrees(-120f32));

        let white = Color::from_rgb8(0xff, 0xff, 0xff);
        let stroke = Stroke {
            style: Style::Solid(white),
            width: 1f32,
            line_cap: LineCap::Square,
            line_join: LineJoin::Bevel,
            line_dash: LineDash {
                segments: &[],
                offset: 0usize,
            },
        };

        let mut frame = Frame::new(bounds.size());

        let content_lock = self.content.clone();
        let content = content_lock
            .lock()
            .expect("locked content in (Visualizer as Program<AppMessage>)::draw");

        match self.display_type {
            crate::DisplayType::Lines => {
                let center = frame.width() as f32 / 2f32;

                let both_data = content.left.iter().zip(content.right.iter());
                for (index, (left_val, right_val)) in both_data.enumerate() {
                    if index as u32 >= self.height {
                        break;
                    }

                    let y = (frame.height() as i32 - index as i32) as f32;
                    let color_shift = RgbHue::from_degrees(360f32 * index as f32 / frame.height());
                    let tip_color: Srgb = red.shift_hue(color_shift).into_color();
                    let color = Color::from_rgb(tip_color.red, tip_color.green, tip_color.blue);

                    let center_point = Point { x: center, y };
                    let left_point = Point {
                        x: center - left_val,
                        y,
                    };
                    let right_point = Point {
                        x: center + right_val,
                        y,
                    };

                    if self.off_center {
                        let mut path_builder = path::Builder::new();
                        path_builder.move_to(left_point);
                        path_builder.line_to(right_point);
                        let path = path_builder.build();
                        frame.stroke(
                            &path,
                            Stroke {
                                style: Style::Gradient(Gradient::Linear(Linear {
                                    start: left_point,
                                    end: right_point,
                                    color_stops: vec![
                                        ColorStop {
                                            offset: 0f32,
                                            color,
                                        },
                                        ColorStop {
                                            offset: 0.5f32,
                                            color: white,
                                        },
                                        ColorStop {
                                            offset: 1f32,
                                            color,
                                        },
                                    ],
                                })),
                                ..stroke
                            },
                        );
                    } else {
                        // do it in two parts, easier that way

                        let mut path_builder = path::Builder::new();
                        path_builder.move_to(left_point);
                        path_builder.line_to(center_point);
                        let path = path_builder.build();
                        frame.stroke(
                            &path,
                            Stroke {
                                style: Style::Gradient(Gradient::Linear(Linear {
                                    start: left_point,
                                    end: center_point,
                                    color_stops: vec![
                                        ColorStop {
                                            offset: 0f32,
                                            color,
                                        },
                                        ColorStop {
                                            offset: 1f32,
                                            color: white,
                                        },
                                    ],
                                })),
                                ..stroke
                            },
                        );

                        let mut path_builder = path::Builder::new();
                        path_builder.move_to(center_point);
                        path_builder.line_to(right_point);
                        let path = path_builder.build();
                        frame.stroke(
                            &path,
                            Stroke {
                                style: Style::Gradient(Gradient::Linear(Linear {
                                    start: center_point,
                                    end: right_point,
                                    color_stops: vec![
                                        ColorStop {
                                            offset: 0f32,
                                            color: white,
                                        },
                                        ColorStop {
                                            offset: 1f32,
                                            color,
                                        },
                                    ],
                                })),
                                ..stroke
                            },
                        );
                    }
                }

                vec![frame.into_geometry()]
            }
            crate::DisplayType::Boxes => todo!(),
            crate::DisplayType::Circle => todo!(),
        }
    }
}
