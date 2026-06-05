use super::{*, leb128_num_traits::*};

// TODO(error_enum): generic support
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LEB128Error<T: NumUnsigned> {
    TooLong { cur: T, shift: u32, byte: u8 },
    TrailingEmptyBytes,
    Read(ReadError),
}

impl<T: NumUnsigned> From<ReadError> for LEB128Error<T> {
    fn from(err: ReadError) -> LEB128Error<T> {
        LEB128Error::Read(err)
    }
}

struct Reader<I> {
    inner: byte_storage::Reader<I>,
    max_lens: MaxLens,
}

impl<B: AsRef<[u8]> + ByteStorage, I: Input<Storage = B>> Reader<I> {
    // begin wrapper impls

    #[inline(always)]
    pub fn new(bytes: B, max_lens: MaxLens) -> Self {
        Self { inner: byte_storage::Reader::new(bytes), max_lens }
    }

    #[inline(always)]
    pub fn byte(&mut self) -> core::result::Result<u8, ReadError> {
        self.inner.read_byte()
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
    pub fn bytes(&mut self, len: usize) -> Result<B> {
        Ok(self.inner.bytes(len)?)
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

    // https://github.com/BillGoldenWater/playground/blob/a9f517d/rust/leb128/src/lib.rs
    // TODO: byte-storage extension?

    pub fn uleb128_inner<T: NumUnsigned>(
        &mut self,
        mut cur: T,
        mut shift: u32,
        last_byte: u8,
    ) -> core::result::Result<T, LEB128Error<T>> {
        if last_byte != 0 {
            cur.shifted_or_assign(last_byte & 0x7F, shift - 7);
        }
        let mut byte = self.byte()?;
        let mut first = last_byte == 0;

        loop {
            cur.shifted_or_assign(byte & 0x7F, shift);

            if byte & 0x80 == 0 {
                if byte == 0 && !first {
                    return Err(LEB128Error::TrailingEmptyBytes);
                }

                break;
            }

            shift += 7;
            if shift >= T::BITS {
                return Err(LEB128Error::TooLong { cur, shift, byte });
            }

            byte = self.byte()?;
            first = false;
        }

        if shift > T::BITS - 7 {
            // extra bits mask
            let mask = !((1 << (T::BITS - shift)) - 1);
            if mask & byte != 0 {
                return Err(LEB128Error::TooLong { cur, shift, byte });
            }
        }

        Ok(cur)
    }

    fn uleb128<N: NumUnsigned>(&mut self) -> Result<N> {
        let res = self.uleb128_inner::<N>(N::from_u8(0), 0, 0);
        match res {
            Ok(n) => Ok(n),
            Err(LEB128Error::Read(err)) => Err(Error::Read(err)),
            Err(LEB128Error::TrailingEmptyBytes) => Err(Error::LEB128TrailingEmptyBytes),
            Err(LEB128Error::TooLong { cur, shift, byte }) => {
                let res2 = self.uleb128_inner::<u128>(cur.to_u128(), shift, byte);

                match res2 {
                    Ok(n2) => Err(Error::LEB128LongerThanTargetType(n2)),
                    Err(LEB128Error::Read(err)) => Err(Error::Read(err)),
                    Err(LEB128Error::TrailingEmptyBytes) => Err(Error::LEB128TrailingEmptyBytes),
                    Err(LEB128Error::TooLong { .. }) => Err(Error::LEB128LongerThan128)
                }
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
