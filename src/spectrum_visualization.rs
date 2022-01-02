use iced::canvas::{path, Cursor, Frame, Geometry, Program, Stroke};
use iced::{Color, Rectangle};
use iced_graphics::Point;

use palette::{convert::IntoColor, Hsv, Hue, Srgb};
use palette::{RgbHue, Saturate};

use crate::Message;

const GRADIENT_GRANULARITY: u32 = 5;

pub struct SpectrumViz<'a> {
    _display_content: crate::DisplayContent,
    display_type: crate::DisplayType,

    to_draw: &'a crate::Sides<Vec<f32>>,

    off_center: bool,
}

impl<'a> SpectrumViz<'a> {
    pub fn new(
        display_content: crate::DisplayContent,
        display_type: crate::DisplayType,
        to_draw: &'a crate::Sides<Vec<f32>>,
        off_center: bool,
    ) -> Self {
        Self {
            _display_content: display_content,
            display_type,
            to_draw,
            off_center,
        }
    }
}

// Then, we implement the `Program` trait
impl<'a> Program<Message> for SpectrumViz<'a> {
    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
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

                let both_data = self.to_draw.left.iter().zip(self.to_draw.right.iter());
                for (index, (left_val, right_val)) in both_data.enumerate() {
                    let y = (frame.height() as i32 - index as i32) as f32;
                    let color_shift = RgbHue::from_degrees(360f32 * index as f32 / frame.height());
                    let tip_color = red.shift_hue(color_shift);

                    let avg = if self.off_center { right_val - left_val } else { 0f32 };

                    let mut draw_side = |distance| {
                        for i in 0..GRADIENT_GRANULARITY {
                            let mut path_builder = path::Builder::new();
                            path_builder.move_to(Point {
                                x: center * (1f32 + (avg + distance) * (i as f32 / GRADIENT_GRANULARITY as f32)),
                                y,
                            });
                            path_builder.line_to(Point {
                                x: center
                                    * (1f32 + (avg + distance) * ((i as f32 + 1f32) / GRADIENT_GRANULARITY as f32)),
                                y,
                            });
                            let path = path_builder.build();

                            let srgb: Srgb = tip_color
                                .desaturate_fixed((GRADIENT_GRANULARITY - i) as f32 / GRADIENT_GRANULARITY as f32)
                                .into_color();
                            let color = Color::new(srgb.red, srgb.green, srgb.blue, 1f32);

                            frame.stroke(&path, Stroke::default().with_color(color).with_width(1f32));
                        }
                    };

                    let dist_from_middle = (right_val + left_val) / 2f32;
                    if self.off_center {
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
