use std::{
    collections::{HashMap, hash_map::Entry},
    path::PathBuf,
    sync::Arc,
};

use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Stretch, Style, SwashCache,
    Weight, Wrap, fontdb,
};
use seruq_ucr::{Color as UcrColor, ElementId};
use tiny_skia::Pixmap;

use crate::{Error, layout::Bounds};

pub(crate) struct TextStyle {
    pub element: ElementId,
    pub bounds: Bounds,
    pub clip: Bounds,
    pub size: f64,
    pub font: Option<PathBuf>,
    pub centered: bool,
    pub color: UcrColor,
}

#[derive(Default)]
pub(crate) struct TextRenderer {
    fonts: HashMap<Option<PathBuf>, FontContext>,
}

struct FontContext {
    system: FontSystem,
    cache: SwashCache,
    family: Option<String>,
    weight: Weight,
    stretch: Stretch,
    style: Style,
}

impl FontContext {
    fn load(path: Option<&PathBuf>, element: ElementId) -> Result<Self, Error> {
        let (system, family, weight, stretch, style) = if let Some(path) = path {
            let bytes = std::fs::read(path).map_err(|source| Error::Asset {
                element,
                path: path.clone(),
                source: Box::new(source),
            })?;
            // A separate database ensures an explicit path selects this file, even if
            // an installed font has the same family name. Each database owns its cache.
            let mut db = fontdb::Database::new();
            let ids = db.load_font_source(fontdb::Source::Binary(Arc::new(bytes)));
            let face =
                ids.first()
                    .and_then(|id| db.face(*id))
                    .ok_or_else(|| Error::InvalidFont {
                        element,
                        path: path.clone(),
                    })?;
            let family = face
                .families
                .first()
                .map(|(name, _)| name.clone())
                .ok_or_else(|| Error::InvalidFont {
                    element,
                    path: path.clone(),
                })?;
            let (weight, stretch, style) = (face.weight, face.stretch, face.style);
            (
                FontSystem::new_with_locale_and_db("en-US".into(), db),
                Some(family),
                weight,
                stretch,
                style,
            )
        } else {
            let system = FontSystem::new();
            if system.db().is_empty() {
                return Err(Error::NoFonts { element });
            }
            (system, None, Weight::NORMAL, Stretch::Normal, Style::Normal)
        };
        Ok(Self {
            system,
            cache: SwashCache::new(),
            family,
            weight,
            stretch,
            style,
        })
    }
}

impl TextRenderer {
    pub fn draw(&mut self, pixmap: &mut Pixmap, text: &str, style: TextStyle) -> Result<(), Error> {
        let Some(clip) = style.bounds.intersect(style.clip) else {
            return Ok(());
        };
        if text.is_empty() {
            return Ok(());
        }
        let size = style.size as f32;
        let line_height = size * 1.2;
        if !size.is_finite()
            || size <= 0.0
            || !line_height.is_finite()
            || style.bounds.x.abs() > f64::from(i32::MAX)
            || style.bounds.y.abs() > f64::from(i32::MAX)
            || !(style.bounds.width as f32).is_finite()
            || !(style.bounds.height as f32).is_finite()
        {
            return Err(Error::InvalidLayout {
                element: style.element,
            });
        }
        let context = match self.fonts.entry(style.font) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let context = FontContext::load(entry.key().as_ref(), style.element)?;
                entry.insert(context)
            }
        };
        let attrs = Attrs::new()
            .family(
                context
                    .family
                    .as_deref()
                    .map(Family::Name)
                    .unwrap_or(Family::SansSerif),
            )
            .weight(context.weight)
            .stretch(context.stretch)
            .style(context.style);
        let mut buffer = Buffer::new(&mut context.system, Metrics::new(size, line_height));
        buffer.set_wrap(Wrap::None);
        buffer.set_size(
            Some(style.bounds.width as f32),
            Some(style.bounds.height as f32),
        );
        buffer.set_text(
            text,
            &attrs,
            Shaping::Advanced,
            style.centered.then_some(Align::Center),
        );
        buffer.shape_until_scroll(&mut context.system, false);
        let text_height = buffer
            .layout_runs()
            .map(|run| run.line_top + run.line_height)
            .fold(0.0_f32, f32::max);
        let origin_x = style.bounds.x.round() as i64;
        let origin_y = (style.bounds.y
            + if style.centered {
                ((style.bounds.height - f64::from(text_height)) * 0.5).max(0.0)
            } else {
                0.0
            })
        .round() as i64;
        let color = Color::rgb(style.color.red, style.color.green, style.color.blue);
        buffer.draw(
            &mut context.system,
            &mut context.cache,
            color,
            |x, y, width, height, color| {
                blend_rect(
                    pixmap,
                    clip,
                    origin_x + i64::from(x),
                    origin_y + i64::from(y),
                    width,
                    height,
                    color,
                );
            },
        );
        Ok(())
    }
}

fn blend_rect(
    pixmap: &mut Pixmap,
    clip: Bounds,
    x: i64,
    y: i64,
    width: u32,
    height: u32,
    color: Color,
) {
    // Clip against pixel centers so no glyph writes outside its element or ancestors.
    let left = x.max((clip.x - 0.5).ceil() as i64).max(0);
    let top = y.max((clip.y - 0.5).ceil() as i64).max(0);
    let right = (x + i64::from(width))
        .min((clip.x + clip.width - 0.5).ceil() as i64)
        .min(i64::from(pixmap.width()));
    let bottom = (y + i64::from(height))
        .min((clip.y + clip.height - 0.5).ceil() as i64)
        .min(i64::from(pixmap.height()));
    let [r, g, b, alpha] = color.as_rgba();
    let alpha = u32::from(alpha);
    let stride = pixmap.width() as usize * 4;
    let pixels = pixmap.data_mut();
    for py in top..bottom {
        for px in left..right {
            let offset = py as usize * stride + px as usize * 4;
            for (channel, value) in [r, g, b].into_iter().enumerate() {
                pixels[offset + channel] = ((u32::from(value) * alpha
                    + u32::from(pixels[offset + channel]) * (255 - alpha)
                    + 127)
                    / 255) as u8;
            }
        }
    }
}
