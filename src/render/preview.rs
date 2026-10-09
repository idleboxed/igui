// SPDX-License-Identifier: BSD-3-Clause
use super::*;
use std::io::Cursor;

pub(super) struct Picture {
    pub(super) position: Point,
    pub(super) surface: Option<imanlib::surface::Surface>,
}

impl Picture {
    fn scaled(width: usize, height: usize, rgb: &[u8]) -> Option<Self> {
        let bounds = rectangle(672, card::PREVIEW_IMAGE_Y, 564, card::PREVIEW_IMAGE_HEIGHT);
        let ratio = (bounds.size.width as f32 / width as f32)
            .min(bounds.size.height as f32 / height as f32);
        let scaled_width = (width as f32 * ratio) as u32;
        let scaled_height = (height as f32 * ratio) as u32;
        // A valid, extremely thin PNG produces an empty scaled image.
        // It is not a decode failure and must not acquire a "No image" label.
        if scaled_width == 0 || scaled_height == 0 {
            return Some(Self {
                position: bounds.top_left,
                surface: None,
            });
        }
        let mut surface = imanlib::surface::Surface::new(scaled_width, scaled_height);
        let colors = (0..scaled_height).flat_map(|y| {
            (0..scaled_width).map(move |x| {
                let i = ((y as usize * height / scaled_height as usize) * width
                    + x as usize * width / scaled_width as usize)
                    * 3;
                Rgb888::new(rgb[i], rgb[i + 1], rgb[i + 2])
            })
        });
        let _ = surface.fill_contiguous(&surface.bounding_box(), colors);
        Some(Self {
            position: Point::new(
                bounds.top_left.x + (bounds.size.width - scaled_width) as i32 / 2,
                bounds.top_left.y + (bounds.size.height - scaled_height) as i32 / 2,
            ),
            surface: Some(surface),
        })
    }
}

fn decode_preview(kind: &Preview, load: &Option<PreviewLoader>) -> Option<Picture> {
    let external;
    let bytes: &[u8] = match kind {
        Preview::Missing => return None,
        Preview::Hash(hash) => {
            external = load.as_ref()?(hash)?;
            &external
        }
    };
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits { bytes: 1024 * 1024 });
    let mut reader = decoder.read_info().ok()?;
    if reader.info().width > 640 || reader.info().height > 480 {
        return None;
    }
    let len = reader.output_buffer_size()?;
    if len > 1024 * 1024 {
        return None;
    }
    let mut rgb = vec![0; len];
    let info = reader.next_frame(&mut rgb).ok()?;
    if info.color_type != png::ColorType::Rgb || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    rgb.truncate(info.buffer_size());
    Picture::scaled(info.width as usize, info.height as usize, &rgb)
}

impl Renderer {
    /// Prepare only the selected screenshot after presenting its loading stage.
    /// The following draw reuses this same one-image cache.
    pub fn prepare_selected(&mut self, app: &App) {
        let Some(game) = app.detail_game() else {
            self.cached_kind = None;
            self.picture = None;
            return;
        };
        if self.cached_kind.as_ref() != Some(&game.preview) {
            self.picture = decode_preview(&game.preview, &self.load_preview);
            self.cached_kind = Some(game.preview.clone());
        }
    }
    pub(super) fn preview<D: DrawTarget<Color = Rgb888>>(
        &mut self,
        app: &App,
        d: &mut D,
    ) -> Result<(), D::Error> {
        fill(
            d,
            rectangle(
                656,
                card::PREVIEW_TOP,
                596,
                (card::PREVIEW_BOTTOM - card::PREVIEW_TOP) as u32,
            ),
            INSET,
        )?;
        self.prepare_selected(app);
        let Some(_) = app.selected_game() else {
            text(
                d,
                app.text("No game selected"),
                862,
                card::PREVIEW_LABEL_Y,
                MUTED,
                false,
            )?;
            return Ok(());
        };
        if let Some(picture) = &self.picture {
            if let Some(surface) = &picture.surface {
                let colors = surface
                    .pixels()
                    .iter()
                    .map(|&p| Rgb888::new((p >> 16) as u8, (p >> 8) as u8, p as u8));
                d.fill_contiguous(&Rectangle::new(picture.position, surface.size()), colors)?;
            }
        } else {
            let label = app.text("No image");
            text(
                d,
                label,
                954 - label.chars().count() as i32 * 5,
                card::PREVIEW_LABEL_Y,
                MUTED,
                false,
            )?;
        }
        Ok(())
    }
}
