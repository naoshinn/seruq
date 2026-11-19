use std::{
    collections::{HashMap, hash_map::Entry},
    path::PathBuf,
};

use seruq_ucr::{AssetPath, Color as UcrColor, ElementId, ElementKind, Ucr};
use tiny_skia::{
    Color, FillRule, FilterQuality, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform,
};

use crate::{
    Error, Frame,
    layout::{Bounds, layout},
    text::{TextRenderer, TextStyle},
};

fn rect(bounds: Bounds) -> Option<Rect> {
    Rect::from_xywh(
        bounds.x as f32,
        bounds.y as f32,
        bounds.width as f32,
        bounds.height as f32,
    )
}

fn raster_color(color: UcrColor) -> Color {
    Color::from_rgba8(color.red, color.green, color.blue, 255)
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
        pressed: Option<ElementId>,
    ) -> Result<Frame, Error> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(Error::InvalidScaleFactor(scale));
        }
        let mut pixmap =
            Pixmap::new(width, height).ok_or(Error::InvalidViewport { width, height })?;
        let data = document.data();
        pixmap.fill(raster_color(
            data.metadata.background().unwrap_or(UcrColor::WHITE),
        ));
        let viewport = Bounds {
            x: 0.0,
            y: 0.0,
            width: f64::from(width),
            height: f64::from(height),
        };
        for placed in layout(document) {
            let id = placed.id;
            let element = &data.elements[&id];
            // Physical bounds.
            let bounds = placed.bounds.scale(scale);
            if ![bounds.x, bounds.y, bounds.width, bounds.height]
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(Error::InvalidLayout { element: id });
            }
            let Some(clip) = placed
                .visible
                .and_then(|visible| visible.scale(scale).intersect(viewport))
            else {
                continue;
            };
            match &element.kind {
                ElementKind::Container { background, .. } => {
                    if let Some(background) = background {
                        fill_rect(&mut pixmap, bounds, clip, raster_color(*background));
                    }
                }
                ElementKind::Text {
                    text,
                    font_size,
                    font,
                    color,
                } => {
                    self.text.draw(
                        &mut pixmap,
                        text,
                        TextStyle {
                            element: id,
                            bounds,
                            clip,
                            size: f64::from(font_size.get()) * scale,
                            font: font
                                .as_ref()
                                .or(data.metadata.default_font())
                                .map(|path| self.asset_base.join(path.as_str())),
                            centered: false,
                            color: color.unwrap_or(UcrColor::BLACK),
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
                    color,
                    background,
                } => {
                    let background = background.unwrap_or(UcrColor::WHITE);
                    let background = if pressed == Some(id) {
                        let darken = |channel| (u16::from(channel) * 208 / 255) as u8;
                        UcrColor::rgb(
                            darken(background.red),
                            darken(background.green),
                            darken(background.blue),
                        )
                    } else {
                        background
                    };
                    draw_control(&mut pixmap, bounds, clip, scale, raster_color(background));
                    self.text.draw(
                        &mut pixmap,
                        label,
                        TextStyle {
                            element: id,
                            bounds: bounds.inset(4.0 * scale),
                            clip,
                            size: f64::from(font_size.get()) * scale,
                            font: font
                                .as_ref()
                                .or(data.metadata.default_font())
                                .map(|path| self.asset_base.join(path.as_str())),
                            centered: true,
                            color: color.unwrap_or(UcrColor::BLACK),
                        },
                    )?;
                }
                ElementKind::TextInput {
                    value,
                    font_size,
                    font,
                    placeholder,
                    color,
                    background,
                } => {
                    draw_control(
                        &mut pixmap,
                        bounds,
                        clip,
                        scale,
                        raster_color(background.unwrap_or(UcrColor::WHITE)),
                    );
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
                            size: f64::from(font_size.get()) * scale,
                            font: font
                                .as_ref()
                                .or(data.metadata.default_font())
                                .map(|path| self.asset_base.join(path.as_str())),
                            centered: false,
                            color: color.unwrap_or(if value.is_empty() {
                                UcrColor::rgb(128, 128, 128)
                            } else {
                                UcrColor::BLACK
                            }),
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
        source: &AssetPath,
        bounds: Bounds,
        clip: Bounds,
    ) -> Result<(), Error> {
        let path = self.asset_base.join(source.as_str());
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
        if let Some(rect) = rect(clip) {
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
    if let Some(rect) = bounds.intersect(clip).and_then(rect) {
        let mut paint = Paint::default();
        paint.set_color(color);
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn draw_control(pixmap: &mut Pixmap, bounds: Bounds, clip: Bounds, scale: f64, background: Color) {
    fill_rect(pixmap, bounds, clip, Color::BLACK);
    fill_rect(pixmap, bounds.inset(scale), clip, background);
}
