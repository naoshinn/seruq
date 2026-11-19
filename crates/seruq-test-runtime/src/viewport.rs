use seruq_ucr::Event;

use crate::Error;

/// Events observed by the application. Viewport events have no element target
/// and are handled by the host, rather than resolved through element actions.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    Element(Event),
    ViewportResized(Viewport),
}

/// Logical dimensions and scale supplied by a window or offscreen host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
    pub scale_factor: f64,
}

impl Viewport {
    pub(crate) fn new(width: u32, height: u32, scale_factor: f64) -> Result<Self, Error> {
        if !scale_factor.is_finite() || scale_factor <= 0.0 {
            return Err(Error::InvalidScaleFactor(scale_factor));
        }
        let logical_width = f64::from(width) / scale_factor;
        let logical_height = f64::from(height) / scale_factor;
        if !logical_width.is_finite() || !logical_height.is_finite() {
            return Err(Error::InvalidScaleFactor(scale_factor));
        }
        Ok(Self {
            width: logical_width,
            height: logical_height,
            scale_factor,
        })
    }
}
