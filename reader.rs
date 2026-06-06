use super::{*, byte_storage::Input, leb128::*};

pub(crate) struct Reader<I> {
    inner: byte_storage::Reader<I>,
    max_lens: MaxLens,
}

impl<B: AsRef<[u8]> + ByteStorage, I: Input<Storage = B>> Reader<I> {
    // begin wrapper impls

    #[inline]
    fn new(bytes: B, max_lens: MaxLens) -> Self {
        Self { inner: byte_storage::Reader::new(bytes), max_lens }
    }

    #[inline]
    pub(crate) fn byte(&mut self) -> Result<u8> {
        Ok(self.inner.read_byte()?)
    }

    #[inline]
    fn finish(self) -> core::result::Result<(), (ReadError, byte_storage::Reader<I>)> {
        self.inner.finish()
    }

    #[inline]
    fn into_rest(self) -> I {
        self.inner.into_rest()
    }

    #[inline]
    fn into_parts(self) -> (I, usize) {
        self.inner.into_parts()
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        Ok(self.inner.read_exact(buf)?)
    }

    #[inline]
    fn bytes(&mut self, len: usize) -> Result<B> {
        Ok(self.inner.bytes(len)?)
    }

    #[inline]
    fn bytes_sized<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.inner.bytes_sized()?)
    }

    // end wrapper impls

    fn finish_with<T>(self, res: Result<T>) -> FullResult<T, B> {
        match res {
            Ok(val) => {
                match self.finish() {
                    Ok(()) => Ok(val),
                    Err((err, reader)) => {
                        let (input, pos) = reader.into_parts();
                        let buf = input.leak();
                        Err(FullError { err: err.into(), buf, pos })
                    }
                }
            },
            Err(err) => {
                let (input, pos) = self.into_parts();
                let buf = input.leak();
                Err(FullError { err, buf, pos })
            }
        }
    }

    fn val(&mut self) -> Result<Value<B>> {
        unimplemented!()
    }
}

// TODO default max lens
impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn decode<I: Input<Storage = B>>(buf: B, max_lens: MaxLens) -> FullResult<Value<B>, B> {
        let mut reader = Reader::<I>::new(buf, max_lens);
        let val = reader.val();
        reader.finish_with(val)
    }

    // cannot return FullResult
    pub fn decode_first_value<I: Input<Storage = B>>(buf: B, max_lens: MaxLens) -> (Result<Value<B>>, B) {
        let mut reader = Reader::<I>::new(buf, max_lens);
        let res = reader.val();
        (res, reader.into_rest().leak())
    }
}
