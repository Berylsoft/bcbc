use super::{*, leb128_num_traits::*};

struct Reader<I> {
    inner: byte_storage::Reader<I>,
}

impl<B: AsRef<[u8]> + ByteStorage, I: Input<Storage = B>> Reader<I> {
    // begin wrapper impls

    #[inline(always)]
    pub fn new(bytes: B) -> Self {
        Self { inner: byte_storage::Reader::new(bytes) }
    }

    #[inline(always)]
    pub fn byte(&mut self) -> Result<u8> {
        Ok(self.inner.read_byte()?)
    }

    #[inline(always)]
    pub fn finish(self) -> core::result::Result<(), (ReadError, byte_storage::Reader<I>)> {
        self.inner.finish()
    }

    #[inline(always)]
    pub fn into_rest(self) -> I {
        self.inner.into_rest()
    }

    #[inline(always)]
    pub fn into_parts(self) -> (I, usize) {
        self.inner.into_parts()
    }

    #[inline(always)]
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        Ok(self.inner.read_exact(buf)?)
    }

    #[inline(always)]
    pub fn bytes(&mut self, sz: usize) -> Result<B> {
        Ok(self.inner.bytes(sz)?)
    }

    #[inline(always)]
    pub fn bytes_sized<const N: usize>(&mut self) -> Result<[u8; N]> {
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

    // https://github.com/BillGoldenWater/playground/blob/2ad09e4/rust/leb128/src/lib.rs
    // TODO: byte-storage extension?

    fn uleb128<N: NumUnsigned>(&mut self) -> Result<N> {
        let mut res = N::from_u8(0);
        let mut shift = 0;
        let mut byte = self.byte()?;

        loop {
            res.shifted_or_assign(byte & 0x7F, shift);
            shift += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if shift >= 128 {
                return Err(Error::LEB128TooLong);
            }

            byte = self.byte()?;
        }

        Ok(res)
    }

    fn sleb128<N: NumSigned>(&mut self) -> Result<N> {
        let mut res = N::UnsignedVariant::from_u8(0);
        let mut shift = 0;
        let mut byte = self.byte()?;

        loop {
            res.shifted_or_assign(byte & 0x7F, shift);
            shift += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if shift >= 128 {
                return Err(Error::LEB128TooLong);
            }

            byte = self.byte()?;
        }

        let mut res = N::from_unsigned(res);

        if shift < N::UnsignedVariant::BITS && byte & 0x40 != 0 {
            res.one_fill_left(shift);
        }

        Ok(res)
    }

    fn val(&mut self) -> Result<Value<B>> {
        unimplemented!()
    }
}

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn decode<I: Input<Storage = B>>(buf: B) -> FullResult<Value<B>, B> {
        let mut reader = Reader::<I>::new(buf);
        let val = reader.val();
        reader.finish_with(val)
    }

    // cannot return FullResult
    pub fn decode_first_value<I: Input<Storage = B>>(buf: B) -> (Result<Value<B>>, B) {
        let mut reader = Reader::<I>::new(buf);
        let res = reader.val();
        (res, reader.into_rest().leak())
    }
}
