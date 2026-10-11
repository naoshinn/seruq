use std::num::NonZeroU32;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use seruq_core::{
    CreateSurfaceError, DrawCommand, PhysicalSize, PresentableFrame, RenderPlan, Renderer, Surface,
    SurfaceTarget,
};
use softbuffer::{Buffer, Context, Surface as SoftbufferSurface};
use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

use crate::SoftbufferRendererError;

/// Draws to targets that expose native display and window handles.
/// Clones of a target must refer to the same display and window.
#[derive(Default)]
pub struct SoftbufferRenderer;

pub struct SoftbufferRenderSurface<T> {
    // Drop the surface before its context.
    surface: SoftbufferSurface<T, T>,
    _context: Context<T>,
    pixmap: Pixmap,
}

pub struct SoftbufferFrame<'a, T> {
    buffer: Buffer<'a, T, T>,
    pixmap: &'a mut Pixmap,
}

impl<T> Renderer<T> for SoftbufferRenderer
where
    T: SurfaceTarget + HasDisplayHandle + HasWindowHandle + Clone,
{
    type Surface = SoftbufferRenderSurface<T>;
    type Error = SoftbufferRendererError;

    fn create_surface(
        &mut self,
        target: T,
        size: PhysicalSize,
    ) -> Result<Self::Surface, CreateSurfaceError<T, Self::Error>> {
        let (width, height) = match nonzero_size(size) {
            Ok(size) => size,
            Err(error) => return Err(CreateSurfaceError::new(target, error)),
        };

        let pixmap = match Pixmap::new(size.width(), size.height()) {
            Some(pixmap) => pixmap,
            None => {
                return Err(CreateSurfaceError::new(
                    target,
                    SoftbufferRendererError::UnsupportedSize,
                ));
            }
        };

        // Keep the original target available for Runtime to retry on any failure.
        let context = match Context::new(target.clone()) {
            Ok(context) => context,
            Err(error) => return Err(CreateSurfaceError::new(target, error.into())),
        };
        let mut surface = match SoftbufferSurface::new(&context, target.clone()) {
            Ok(surface) => surface,
            Err(error) => return Err(CreateSurfaceError::new(target, error.into())),
        };
        if let Err(error) = surface.resize(width, height) {
            return Err(CreateSurfaceError::new(target, error.into()));
        }

        Ok(SoftbufferRenderSurface {
            surface,
            _context: context,
            pixmap,
        })
    }

    fn render(
        &mut self,
        frame: &mut <Self::Surface as Surface>::Frame<'_>,
        plan: &RenderPlan,
    ) -> Result<(), Self::Error> {
        render_pixmap(frame.pixmap, plan)
    }
}

impl<T: HasDisplayHandle + HasWindowHandle> Surface for SoftbufferRenderSurface<T> {
    type Error = SoftbufferRendererError;
    type Frame<'a>
        = SoftbufferFrame<'a, T>
    where
        Self: 'a;

    fn resize(&mut self, size: PhysicalSize) -> Result<(), Self::Error> {
        let (width, height) = nonzero_size(size)?;
        let pixmap = Pixmap::new(size.width(), size.height())
            .ok_or(SoftbufferRendererError::UnsupportedSize)?;
        self.surface.resize(width, height)?;
        self.pixmap = pixmap;
        Ok(())
    }

    fn acquire(&mut self) -> Result<Option<Self::Frame<'_>>, Self::Error> {
        let buffer = self.surface.buffer_mut()?;
        Ok(Some(SoftbufferFrame {
            buffer,
            pixmap: &mut self.pixmap,
        }))
    }
}

impl<T: HasDisplayHandle + HasWindowHandle> PresentableFrame for SoftbufferFrame<'_, T> {
    type Error = SoftbufferRendererError;

    fn present(self) -> Result<(), Self::Error> {
        let Self { mut buffer, pixmap } = self;
        copy_pixels(pixmap.data(), &mut buffer)?;
        buffer.present()?;
        Ok(())
    }
}

fn nonzero_size(size: PhysicalSize) -> Result<(NonZeroU32, NonZeroU32), SoftbufferRendererError> {
    let width = NonZeroU32::new(size.width()).ok_or(SoftbufferRendererError::EmptySize)?;
    let height = NonZeroU32::new(size.height()).ok_or(SoftbufferRendererError::EmptySize)?;
    Ok((width, height))
}

fn render_pixmap(pixmap: &mut Pixmap, plan: &RenderPlan) -> Result<(), SoftbufferRendererError> {
    pixmap.fill(Color::WHITE);

    for command in plan.commands() {
        match command {
            DrawCommand::FillRect { bounds, color } => {
                if bounds.is_empty() {
                    continue;
                }
                let origin = bounds.origin();
                let size = bounds.size();
                let rect = Rect::from_xywh(origin.x(), origin.y(), size.width(), size.height())
                    .ok_or(SoftbufferRendererError::InvalidRect)?;
                let color = Color::from_rgba(color.r(), color.g(), color.b(), 1.0)
                    .expect("seruq-core validates Color channels");
                let mut paint = Paint::default();
                paint.set_color(color);
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
        }
    }

    Ok(())
}

fn copy_pixels(rgba: &[u8], rgb: &mut [u32]) -> Result<(), SoftbufferRendererError> {
    if rgba.len() != rgb.len().saturating_mul(4) {
        return Err(SoftbufferRendererError::BufferSizeMismatch);
    }

    // The white clear makes every output pixel opaque, including antialiased edges.
    for (target, pixel) in rgb.iter_mut().zip(rgba.as_chunks::<4>().0.iter()) {
        *target = (u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2]);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use seruq_core::{Color, DrawCommand, Point, Rect, RenderPlan, Size};

    use super::*;

    #[test]
    fn empty_plan_clears_to_white() {
        let mut pixmap = Pixmap::new(2, 2).unwrap();
        render_pixmap(&mut pixmap, &RenderPlan::new(Vec::new())).unwrap();

        assert!(
            pixmap
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == [255, 255, 255, 255])
        );
    }

    #[test]
    fn fill_rect_draws_over_white_background() {
        let mut pixmap = Pixmap::new(4, 4).unwrap();
        let bounds = Rect::new(Point::new(1.0, 1.0).unwrap(), Size::new(2.0, 2.0).unwrap());
        let plan = RenderPlan::new(vec![DrawCommand::FillRect {
            bounds,
            color: Color::new(1.0, 0.0, 0.0).unwrap(),
        }]);

        render_pixmap(&mut pixmap, &plan).unwrap();

        let pixel = |x: usize, y: usize| &pixmap.data()[(y * 4 + x) * 4..][..4];
        assert_eq!(pixel(0, 0), [255, 255, 255, 255]);
        assert_eq!(pixel(1, 1), [255, 0, 0, 255]);
        assert_eq!(pixel(2, 2), [255, 0, 0, 255]);
        assert_eq!(pixel(3, 3), [255, 255, 255, 255]);
    }

    #[test]
    fn copies_rgba_pixels_to_softbuffer_format() {
        let rgba = [0x12, 0x34, 0x56, 0xff, 0xab, 0xcd, 0xef, 0xff];
        let mut rgb = [0; 2];

        copy_pixels(&rgba, &mut rgb).unwrap();

        assert_eq!(rgb, [0x00123456, 0x00abcdef]);
    }

    #[test]
    fn rejects_mismatched_pixel_counts() {
        let mut rgb = [0; 2];

        assert!(matches!(
            copy_pixels(&[255, 255, 255, 255], &mut rgb),
            Err(SoftbufferRendererError::BufferSizeMismatch)
        ));
    }
}
