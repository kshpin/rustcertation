use iced::canvas::{path, Cursor, Frame, Geometry, Program, Stroke};
use iced::{Color, Rectangle};
use iced_graphics::Point;

use crate::Message;

pub struct SpectrumViz {
    _display_content: crate::DisplayContent,
    display_type: crate::DisplayType,

    to_draw: crate::Sides<Vec<f32>>,
}

impl SpectrumViz {
    pub fn new(
        display_content: crate::DisplayContent,
        display_type: crate::DisplayType,
        to_draw: crate::Sides<Vec<f32>>,
    ) -> Self {
        Self {
            _display_content: display_content,
            display_type,
            to_draw,
        }
    }
}

// Then, we implement the `Program` trait
impl Program<Message> for SpectrumViz {
    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        //println!("{}", bounds.size());

        // We prepare a new `Frame`
        let mut frame = Frame::new(bounds.size());

        match self.display_type {
            crate::DisplayType::Lines => {
                let middle = frame.width() as f32 / 2f32;

                let both_data = self.to_draw.left.iter().zip(self.to_draw.right.iter());

                let mut path_builder = path::Builder::new();
                for (index, (left_val, right_val)) in both_data.enumerate() {
                    let y = (frame.height() as i32 - index as i32) as f32;

                    path_builder.move_to(Point {
                        x: middle - left_val * middle,
                        y,
                    });

                    path_builder.line_to(Point {
                        x: middle + right_val * middle,
                        y,
                    });
                }

                let path = path_builder.build();
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(Color::from_rgb8(0, 255, 255))
                        .with_width(1f32),
                );
                vec![frame.into_geometry()]
            }
            crate::DisplayType::Boxes => todo!(),
            crate::DisplayType::Circle => todo!(),
        }
    }
}
