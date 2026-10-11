use std::sync::Arc;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use seruq_core::{PhysicalSize, Renderer, SurfaceTarget};
use seruq_renderer_softbuffer::{SoftbufferRenderer, SoftbufferRendererError};

// A target supplied by a different platform, whose native handles are unavailable.
#[derive(Clone)]
struct UnavailableTarget {
    identity: Arc<()>,
}

impl SurfaceTarget for UnavailableTarget {}

impl HasDisplayHandle for UnavailableTarget {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Err(HandleError::Unavailable)
    }
}

impl HasWindowHandle for UnavailableTarget {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Err(HandleError::Unavailable)
    }
}

#[test]
fn accepts_custom_target_and_returns_it_when_handle_is_unavailable() {
    let identity = Arc::new(());
    let target = UnavailableTarget {
        identity: Arc::clone(&identity),
    };

    let error = SoftbufferRenderer
        .create_surface(target, PhysicalSize::new(4, 4))
        .err()
        .expect("unavailable handles must fail surface creation");
    let (target, error) = error.into_parts();

    assert!(Arc::ptr_eq(&target.identity, &identity));
    assert_eq!(Arc::strong_count(&identity), 2);
    assert!(matches!(
        error,
        SoftbufferRendererError::Softbuffer(softbuffer::SoftBufferError::RawWindowHandle(
            HandleError::Unavailable
        ))
    ));
}
