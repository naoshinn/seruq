use std::error::Error;

use seruq_core::{Color, Platform, Point, Rect, Size, Ucr};
use seruq_platform_winit::{WinitPlatform, WinitSurfaceTarget};
use seruq_renderer_softbuffer::SoftbufferRenderer;
use seruq_runtime::DefaultRuntime;

fn main() -> Result<(), Box<dyn Error>> {
    let bounds = Rect::new(Point::new(100.0, 100.0)?, Size::new(300.0, 200.0)?);
    let ucr = Ucr::rect(bounds, Color::new(0.2, 0.4, 0.8)?);
    let mut runtime = DefaultRuntime::<WinitSurfaceTarget, _>::new(ucr, SoftbufferRenderer);

    WinitPlatform.run(&mut runtime)?;
    Ok(())
}
