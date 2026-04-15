pub trait RawFrame {
    fn as_bytes(&self) -> &[u8];
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
    fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }
}

pub trait RawFrameMut: RawFrame {
    fn as_bytes_mut(&mut self) -> &mut [u8];
}

pub trait ValidateFrame {
    type Error;

    fn validate(&self) -> Result<(), Self::Error>;
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

pub trait AsImmutableFrame<'a> {
    type Immutable;

    fn as_frame(&'a self) -> Self::Immutable;
}
