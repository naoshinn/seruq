use crate::PhysicalSize;

pub trait SurfaceTarget {}

pub trait PresentableFrame {
    type Error;

    fn present(self) -> Result<(), Self::Error>;
}

pub trait Surface {
    type Error;
    type Frame<'a>: PresentableFrame<Error = Self::Error>
    where
        Self: 'a;

    fn resize(&mut self, size: PhysicalSize) -> Result<(), Self::Error>;

    fn acquire(&mut self) -> Result<Option<Self::Frame<'_>>, Self::Error>;
}
