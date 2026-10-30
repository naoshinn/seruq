use std::{
    collections::{HashMap, hash_map::Entry},
    path::PathBuf,
};

use seruq_ucr::{ElementId, ElementKind, Ucr};
use tiny_skia::{
    Color, FillRule, FilterQuality, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform,
};

use crate::{
    Error, Frame,
    text::{TextRenderer, TextStyle},
};

/// Physical bounds. f64 keeps accumulated logical coordinates from overflowing f32.
#[derive(Clone, Copy)]
pub(crate) struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn intersect(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }

    pub fn inset(self, amount: f64) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: (self.width - 2.0 * amount).max(0.0),
            height: (self.height - 2.0 * amount).max(0.0),
        }
    }

    fn rect(self) -> Option<Rect> {
        Rect::from_xywh(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        )
    }
}

pub(crate) struct Renderer {
    asset_base: PathBuf,
    images: HashMap<PathBuf, Pixmap>,
    text: TextRenderer,
}

impl Renderer {
    pub fn new(asset_base: PathBuf) -> Self {
        Self {
            asset_base,
            images: HashMap::new(),
            text: TextRenderer::default(),
        }
    }

    pub fn render(
        &mut self,
        document: &Ucr,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Result<Frame, Error> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(Error::InvalidScaleFactor(scale));
        }
        let mut pixmap =
            Pixmap::new(width, height).ok_or(Error::InvalidViewport { width, height })?;
        pixmap.fill(Color::WHITE);
        let viewport = Bounds {
            x: 0.0,
            y: 0.0,
            width: f64::from(width),
            height: f64::from(height),
        };
        let data = document.data();
        let mut stack = vec![(data.root, 0.0, 0.0, viewport)];
        while let Some((id, parent_x, parent_y, parent_clip)) = stack.pop() {
            // Ucr guarantees that every reachable reference exists and has a unique parent.
            let element = &data.elements[&id];
            let x = parent_x + f64::from(element.position.x());
            let y = parent_y + f64::from(element.position.y());
            let bounds = Bounds {
                x: x * scale,
                y: y * scale,
                width: f64::from(element.size.width()) * scale,
                height: f64::from(element.size.height()) * scale,
            };
            if ![bounds.x, bounds.y, bounds.width, bounds.height]
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(Error::InvalidLayout { element: id });
            }
            let Some(clip) = parent_clip.intersect(bounds) else {
                continue;
            };
            match &element.kind {
                ElementKind::Container { children } => {
                    // Push in reverse so each subtree is completed in children order.
                    stack.extend(children.iter().rev().map(|&child| (child, x, y, clip)));
                }
                ElementKind::Text {
                    text,
                    font_size,
                    font,
                } => {
                    self.text.draw(
                        &mut pixmap,
                        text,
                        TextStyle {
                            element: id,
                            bounds,
                            clip,
                            size: f64::from(*font_size) * scale,
                            font: font
                                .as_deref()
                                .or(data.metadata.default_font())
                                .map(|path| self.asset_base.join(path)),
                            centered: false,
                            placeholder: false,
                        },
                    )?;
                }
                ElementKind::Image { source } => {
                    self.draw_image(&mut pixmap, id, source, bounds, clip)?
                }
                ElementKind::Button {
                    label,
                    font_size,
                    font,
                } => {
                    draw_control(&mut pixmap, bounds, clip, scale);
                    self.text.draw(
                        &mut pixmap,
                        label,
                        TextStyle {
                            element: id,
                            bounds: bounds.inset(4.0 * scale),
                            clip,
                            size: f64::from(*font_size) * scale,
                            font: font
                                .as_deref()
                                .or(data.metadata.default_font())
                                .map(|path| self.asset_base.join(path)),
                            centered: true,
                            placeholder: false,
                        },
                    )?;
                }
                ElementKind::TextInput {
                    value,
                    font_size,
                    font,
                    placeholder,
                } => {
                    draw_control(&mut pixmap, bounds, clip, scale);
                    let text = if value.is_empty() {
                        placeholder.as_deref().unwrap_or("")
                    } else {
                        value
                    };
                    self.text.draw(
                        &mut pixmap,
                        text.lines().next().unwrap_or(""),
                        TextStyle {
                            element: id,
                            bounds: bounds.inset(4.0 * scale),
                            clip,
                            size: f64::from(*font_size) * scale,
                            font: font
                                .as_deref()
                                .or(data.metadata.default_font())
                                .map(|path| self.asset_base.join(path)),
                            centered: false,
                            placeholder: value.is_empty(),
                        },
                    )?;
                }
                ElementKind::Geometry => {
                    return Err(Error::UnsupportedElement {
                        element: id,
                        kind: "Geometry",
                    });
                }
            }
        }
        Ok(Frame { pixmap })
    }

    fn draw_image(
        &mut self,
        pixmap: &mut Pixmap,
        id: ElementId,
        source: &str,
        bounds: Bounds,
        clip: Bounds,
    ) -> Result<(), Error> {
        let path = self.asset_base.join(source);
        let image = match self.images.entry(path) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let image = Pixmap::load_png(entry.key()).map_err(|source| Error::Asset {
                    element: id,
                    path: entry.key().clone(),
                    source: Box::new(source),
                })?;
                entry.insert(image)
            }
        };
        let scale = (bounds.width / f64::from(image.width()))
            .min(bounds.height / f64::from(image.height()));
        let tx = bounds.x + (bounds.width - f64::from(image.width()) * scale) * 0.5;
        let ty = bounds.y + (bounds.height - f64::from(image.height()) * scale) * 0.5;
        if ![scale as f32, tx as f32, ty as f32]
            .iter()
            .all(|value| value.is_finite())
            || scale as f32 <= 0.0
        {
            return Err(Error::InvalidLayout { element: id });
        }
        let mut mask =
            Mask::new(pixmap.width(), pixmap.height()).ok_or(Error::InvalidViewport {
                width: pixmap.width(),
                height: pixmap.height(),
            })?;
        if let Some(rect) = clip.rect() {
            mask.fill_path(
                &PathBuilder::from_rect(rect),
                FillRule::Winding,
                true,
                Transform::identity(),
            );
        }
        pixmap.draw_pixmap(
            0,
            0,
            image.as_ref(),
            &PixmapPaint {
                quality: FilterQuality::Bilinear,
                ..PixmapPaint::default()
            },
            Transform::from_row(scale as f32, 0.0, 0.0, scale as f32, tx as f32, ty as f32),
            Some(&mask),
        );
        Ok(())
    }
}

fn fill_rect(pixmap: &mut Pixmap, bounds: Bounds, clip: Bounds, color: Color) {
    if let Some(rect) = bounds.intersect(clip).and_then(Bounds::rect) {
        let mut paint = Paint::default();
        paint.set_color(color);
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn draw_control(pixmap: &mut Pixmap, bounds: Bounds, clip: Bounds, scale: f64) {
    fill_rect(pixmap, bounds, clip, Color::BLACK);
    fill_rect(pixmap, bounds.inset(scale), clip, Color::WHITE);
}
