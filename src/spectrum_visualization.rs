use std::iter;

use iced::widget::canvas::{path, Canvas, Cursor, Frame, Geometry, Program, Stroke};
use iced::widget::Container;
use iced::{Color, Element, Length, Rectangle, Theme};
use iced_graphics::Point;

use palette::{convert::IntoColor, Hsv, Hue, Srgb};
use palette::{RgbHue, Saturate};
use ringbuffer::RingBufferExt;
use spectrum_analyzer::{samples_fft_to_spectrum, windows, FrequencyLimit};

use crate::sound_proxy::Clip;
use crate::sound_transformer::SoundTransformer;
use crate::{AppMessage, ContentType, Sides};

const GRADIENT_GRANULARITY: u32 = 5;

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

    content: crate::Sides<Vec<f32>>,

    sound_transformer: SoundTransformer,

    off_center: bool,
}

impl Visualizer {
    pub fn new(
        width: u32,
        height: u32,
        content_type: crate::ContentType,
        display_type: crate::DisplayType,
        content: crate::Sides<Vec<f32>>,
        off_center: bool,
    ) -> Self {
        Self {
            width,
            height,
            content_type,
            display_type,
            content,
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

                self.content = if let ContentType::Raw = self.content_type {
                    raw
                } else {
                    Sides {
                        left: process(&raw.left, &self.content.left),
                        right: process(&raw.right, &self.content.right),
                    }
                };
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
        //println!("{}", bounds.size());

        // We prepare a new `Frame`
        let mut frame = Frame::new(bounds.size());

        // TODO: play around with colors more
        // start at green, which is brighter than red, then rotate back to red, which doesn't actually yield back red :/
        let red = Hsv::new(0f32, 1f32, 1f32);
        //let red = yellow.shift_hue(LabHue::from_degrees(-120f32));

        match self.display_type {
            crate::DisplayType::Lines => {
                let center = frame.width() as f32 / 2f32;

                let both_data = self.content.left.iter().zip(self.content.right.iter());
                for (index, (left_val, right_val)) in both_data.enumerate() {
                    let y = (frame.height() as i32 - index as i32) as f32;
                    let color_shift = RgbHue::from_degrees(360f32 * index as f32 / frame.height());
                    let tip_color = red.shift_hue(color_shift);

                    let avg = if self.off_center {
                        right_val - left_val
                    } else {
                        0f32
                    };

                    let mut draw_side = |distance| {
                        for i in 0..GRADIENT_GRANULARITY {
                            let mut path_builder = path::Builder::new();
                            path_builder.move_to(Point {
                                x: center
                                    * (1f32
                                        + (avg + distance)
                                            * (i as f32 / GRADIENT_GRANULARITY as f32)),
                                y,
                            });
                            path_builder.line_to(Point {
                                x: center
                                    * (1f32
                                        + (avg + distance)
                                            * ((i as f32 + 1f32) / GRADIENT_GRANULARITY as f32)),
                                y,
                            });
                            let path = path_builder.build();

                            let srgb: Srgb = tip_color
                                .desaturate_fixed(
                                    (GRADIENT_GRANULARITY - i) as f32 / GRADIENT_GRANULARITY as f32,
                                )
                                .into_color();
                            let color = Color::new(srgb.red, srgb.green, srgb.blue, 1f32);

                            frame.stroke(
                                &path,
                                Stroke::default().with_color(color).with_width(1f32),
                            );
                        }
                    };

                    if self.off_center {
                        let dist_from_middle = (right_val + left_val) / 2f32;
                        draw_side(-dist_from_middle);
                        draw_side(dist_from_middle);
                    } else {
                        draw_side(-*left_val);
                        draw_side(*right_val);
                    }
                }

                vec![frame.into_geometry()]
            }
            crate::DisplayType::Boxes => todo!(),
            crate::DisplayType::Circle => todo!(),
        }
    }
}
