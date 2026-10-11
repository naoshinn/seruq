#[derive(Debug, thiserror::Error)]
pub enum SoftbufferRendererError {
    #[error(transparent)]
    Softbuffer(#[from] softbuffer::SoftBufferError),
    #[error("surface size must be nonzero")]
    EmptySize,
    #[error("surface size is unsupported by tiny-skia")]
    UnsupportedSize,
    #[error("rectangle coordinates are unsupported by tiny-skia")]
    InvalidRect,
    #[error("softbuffer and tiny-skia pixel counts differ")]
    BufferSizeMismatch,
}
