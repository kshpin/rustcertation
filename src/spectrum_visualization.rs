use iced::canvas::{path, Cursor, Frame, Geometry, Program, Stroke};
use iced::{Color, Rectangle};
use iced_graphics::Point;

use palette::{convert::IntoColor, Hsv, Hue, LabHue, Lch, Srgb};

use crate::Message;

pub struct SpectrumViz<'a> {
    _display_content: crate::DisplayContent,
    display_type: crate::DisplayType,

    to_draw: &'a crate::Sides<Vec<f32>>,
}

impl<'a> SpectrumViz<'a> {
    pub fn new(
        display_content: crate::DisplayContent,
        display_type: crate::DisplayType,
        to_draw: &'a crate::Sides<Vec<f32>>,
    ) -> Self {
        Self {
            _display_content: display_content,
            display_type,
            to_draw,
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
        let yellow: Lch = Hsv::new(120f32, 1f32, 1f32).into_color();
        let red = yellow.shift_hue(LabHue::from_degrees(-120f32));

        match self.display_type {
            crate::DisplayType::Lines => {
                let middle = frame.width() as f32 / 2f32;

                let both_data = self.to_draw.left.iter().zip(self.to_draw.right.iter());
                for (index, (left_val, right_val)) in both_data.enumerate() {
                    let y = (frame.height() as i32 - index as i32) as f32;
                    let color = LabHue::from_degrees(360f32 * index as f32 / frame.height());

                    let mut path_builder = path::Builder::new();
                    path_builder.move_to(Point {
                        x: middle - left_val * middle,
                        y,
                    });
                    path_builder.line_to(Point {
                        x: middle + right_val * middle,
                        y,
                    });
                    let path = path_builder.build();

                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_color({
                                let srgb: Srgb = red.shift_hue(color).into_color();
                                Color::new(srgb.red, srgb.green, srgb.blue, 1f32)
                            })
                            .with_width(1f32),
                    );
                }

                vec![frame.into_geometry()]
            }
            crate::DisplayType::Boxes => todo!(),
            crate::DisplayType::Circle => todo!(),
        }
    }
}
