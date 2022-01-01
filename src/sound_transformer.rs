pub struct SoundTransformer {
    norm: bool,
    full_norm: bool,
    norm_scale: f32,

    smooth: bool,
    flash_flood: bool,

    moving_avg_range: u32,
    moving_avg_k: f32,
}

impl Default for SoundTransformer {
    fn default() -> Self {
        let moving_avg_range = 10u32;
        let moving_avg_k = get_moving_avg_coefficient(moving_avg_range);

        Self {
            norm: true,
            full_norm: false,
            norm_scale: 1f32,

            smooth: true,
            flash_flood: true,

            moving_avg_range,
            moving_avg_k,
        }
    }
}

impl SoundTransformer {
    pub fn apply(&self, old: f32, new: f32, index: f32) -> f32 {
        let base_scale = 0.25f32;
        let power = 1.5f32;

        self.smoothen(old, self.normalize((base_scale * new).abs().powf(power), index))
    }

    fn normalize(&self, val: f32, index: f32) -> f32 {
        let power = 0.7f32;
        let scale = 0.000000000015f32;
        let full_scale = 0.02f32;

        if self.norm {
            if self.full_norm {
                val * (index + 1f32) * full_scale
            } else {
                val * (index + 1f32).powf(power) * scale * self.norm_scale
            }
        } else {
            val
        }
    }

    fn smoothen(&self, old: f32, new: f32) -> f32 {
        if self.smooth {
            if self.flash_flood && new > old {
                new
            } else {
                new * self.moving_avg_k + old * (1f32 - self.moving_avg_k)
            }
        } else {
            new
        }
    }

    // mutators ------------------------------------------------------------------------------------

    pub fn toggle_norm(&mut self) {
        self.norm = !self.norm;
    }

    pub fn toggle_smooth(&mut self) {
        self.smooth = !self.smooth;
    }

    pub fn toggle_flash_flood(&mut self) {
        self.flash_flood = !self.flash_flood;
    }

    pub fn shift_norm_scale(&mut self, factor: f32) {
        self.norm_scale *= factor;
    }

    pub fn shift_moving_avg_range(&mut self, val: i32, debug: bool) {
        self.set_moving_avg_range(if val < 0 && val.abs() as u32 > self.moving_avg_range {
            0u32
        } else {
            self.moving_avg_range + val as u32
        });

        if debug {
            println!("Moving average range: {}", self.moving_avg_range);
        }
    }

    fn set_moving_avg_range(&mut self, val: u32) {
        self.moving_avg_range = val;
        self.moving_avg_k = get_moving_avg_coefficient(val);
    }
}

fn get_moving_avg_coefficient(range: u32) -> f32 {
    2f32 / (1f32 + range as f32)
}
