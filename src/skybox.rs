use std::path::Path;

use image::ImageReader;

use crate::framebuffer::Framebuffer;

pub struct Skybox {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

impl Skybox {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, image::ImageError> {
        let img = ImageReader::open(path)?.decode()?.to_rgb8();
        let (width, height) = img.dimensions();
        let mut pixels = Vec::with_capacity((width * height) as usize);
        let raw = img.into_raw();

        for chunk in raw.chunks(3) {
            let r = chunk.get(0).copied().unwrap_or(0);
            let g = chunk.get(1).copied().unwrap_or(0);
            let b = chunk.get(2).copied().unwrap_or(0);
            pixels.push(((r as u32) << 16) | ((g as u32) << 8) | (b as u32));
        }

        Ok(Skybox {
            width: width as usize,
            height: height as usize,
            pixels,
        })
    }

    pub fn draw(&self, framebuffer: &mut Framebuffer) {
        if self.pixels.is_empty() {
            return;
        }

        for y in 0..framebuffer.height {
            let src_y = y * self.height / framebuffer.height;
            for x in 0..framebuffer.width {
                let src_x = x * self.width / framebuffer.width;
                let color = self.pixels[src_y * self.width + src_x];
                framebuffer.plot_overlay(x as i32, y as i32, color);
            }
        }
    }
}
