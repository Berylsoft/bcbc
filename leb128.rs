// https://github.com/BillGoldenWater/playground/blob/1799908/rust/leb128/src/lib.rs
// TODO: byte-storage extension?

// region: num traits

#[cfg(not(feature = "text-writer"))]
pub trait NumBase: Copy {}

#[cfg(feature = "text-writer")]
pub trait NumBase: Copy + itoa::Integer {}

pub trait NumUnsigned: NumBase {
    const BITS: u32;

    fn from_u8(value: u8) -> Self;

    fn trunc_u8(&self) -> u8;
    fn all_zero(&self) -> bool;
    fn all_one(&self) -> bool;
    fn shr_assign(&mut self, rhs: u32);
    fn sar_assign(&mut self, rhs: u32);
    fn shifted_or_assign(&mut self, rhs: u8, shift: u32);

    fn to_u128(&self) -> u128;
}

pub trait NumSigned: NumBase {
    type UnsignedVariant: NumUnsigned;

    fn as_unsigned(&self) -> Self::UnsignedVariant;
    fn from_unsigned(value: Self::UnsignedVariant) -> Self;
    fn one_fill_left(&mut self, right: u32);
}

macro_rules! impl_num {
    ($ty:ty, $signed_ty:ty) => {
        impl NumBase for $ty {}

        impl NumUnsigned for $ty {
            const BITS: u32 = <$ty>::BITS;

            #[inline]
            fn from_u8(value: u8) -> $ty {
                value as $ty
            }

            #[inline]
            fn trunc_u8(&self) -> u8 {
                *self as u8
            }

            #[inline]
            fn all_zero(&self) -> bool {
                *self == 0
            }

            #[inline]
            fn all_one(&self) -> bool {
                *self == <$ty>::MAX
            }

            #[inline]
            fn shr_assign(&mut self, rhs: u32) {
                *self >>= rhs;
            }

            #[inline]
            fn sar_assign(&mut self, rhs: u32) {
                *self = ((*self as $signed_ty) >> rhs) as $ty;
            }

            #[inline]
            fn shifted_or_assign(&mut self, rhs: u8, shift: u32) {
                *self |= (rhs as $ty) << shift;
            }

            #[inline]
            fn to_u128(&self) -> u128 {
                *self as u128
            }
        }

        impl NumBase for $signed_ty {}

        impl NumSigned for $signed_ty {
            type UnsignedVariant = $ty;

            #[inline]
            fn as_unsigned(&self) -> Self::UnsignedVariant {
                *self as Self::UnsignedVariant
            }

            #[inline]
            fn from_unsigned(value: Self::UnsignedVariant) -> Self {
                value as $signed_ty
            }

            #[inline]
            fn one_fill_left(&mut self, right: u32) {
                *self = (*self as $ty | <$ty>::MAX.wrapping_shl(right))
                    as $signed_ty;
            }
        }
    };
}

impl_num!(u128, i128);
impl_num!(u64, i64);
impl_num!(u32, i32);
impl_num!(u16, i16);
impl_num!(u8, i8);

const _: () = assert_size_length();
const fn assert_size_length() {
    if usize::BITS > 128 || isize::BITS > 128 {
        panic!("archs that size length larger than 128-bit is not supported currently");
        // TODO needs uN supported in Rust and try_into() on that archs
    }
}

impl_num!(usize, isize);

// endregion

// region: decode state & error

#[derive(Clone, Debug, PartialEq, Eq)]
enum Error<T: NumUnsigned> {
    TooLong(DecodeState<T>),
    Others(crate::Error),
}

impl<T: NumUnsigned> From<crate::Error> for Error<T> {
    fn from(err: crate::Error) -> Error<T> {
        Error::Others(err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecodeState<T: NumUnsigned> {
    cur: T,
    shift: u32,
    byte: u8,
}

impl<T: NumUnsigned> DecodeState<T> {
    // TODO into_super like orig
    fn into_128(self) -> DecodeState<u128> {
        DecodeState {
            cur: self.cur.to_u128(),
            shift: self.shift,
            byte: self.byte,
        }
    }
}

impl<T: NumUnsigned> Default for DecodeState<T> {
    fn default() -> Self {
        Self {
            cur: T::from_u8(0),
            shift: 0,
            byte: 0,
        }
    }
}

// endregion

// region: reader

use crate::{byte_storage::{ByteStorage, Input}, reader::Reader};

impl<B: AsRef<[u8]> + ByteStorage, I: Input<Storage = B>> Reader<I> {
    fn uleb128_inner<T: NumUnsigned>(
        &mut self,
        state: Option<DecodeState<T>>,
    ) -> Result<T, Error<T>> {
        let DecodeState {
            mut cur,
            mut shift,
            mut byte,
        } = state.unwrap_or_default();
        let mut first = byte == 0;
        if first {
            byte = self.byte()?;
        }

        loop {
            cur.shifted_or_assign(byte & 0x7F, shift);

            if byte & 0x80 == 0 {
                if byte == 0 && !first {
                    return Err(crate::Error::LEB128TrailingEmptyBytes.into());
                }

                break;
            }

            shift += 7;
            if shift >= T::BITS {
                return Err(Error::TooLong(DecodeState {
                    cur,
                    shift: shift - 7,
                    byte,
                }));
            }

            byte = self.byte()?;
            first = false;
        }

        if shift > T::BITS - 7 {
            // extra bits mask
            let mask = !((1 << (T::BITS - shift)) - 1);
            if mask & byte != 0 {
                return Err(Error::TooLong(DecodeState {
                    cur,
                    shift,
                    byte,
                }));
            }
        }

        Ok(cur)
    }

    pub(crate) fn uleb128<N: NumUnsigned>(&mut self) -> crate::Result<N> {
        let res = self.uleb128_inner::<N>(None);
        match res {
            Ok(n) => Ok(n),
            Err(Error::Others(err)) => Err(err),
            Err(Error::TooLong(state)) => {
                let res2 = self.uleb128_inner::<u128>(Some(state.into_128()));

                match res2 {
                    Ok(n) => Err(crate::Error::ULEB128LongerThanTargetType(n, core::any::type_name::<N>())),
                    Err(Error::Others(err)) => Err(err),
                    Err(Error::TooLong { .. }) => Err(crate::Error::LEB128LongerThan128)
                }
            }
        }
    }

    fn sleb128_inner<T: NumSigned>(
        &mut self,
        state: Option<DecodeState<T::UnsignedVariant>>,
    ) -> core::result::Result<T, Error<T::UnsignedVariant>> {
        let bits = T::UnsignedVariant::BITS;
        let DecodeState {
            mut cur,
            mut shift,
            mut byte,
        } = state.unwrap_or_default();

        let mut last_byte = 0;
        let mut first = byte == 0;
        if first {
            byte = self.byte()?;
        }

        loop {
            cur.shifted_or_assign(byte & 0x7F, shift);

            if byte & 0x80 == 0 {
                if !first {
                    let pos = byte == 0 && last_byte & 0x40 == 0;
                    let neg = byte == 0x7F && last_byte & 0x40 != 0;
                    if pos || neg {
                        return Err(crate::Error::LEB128TrailingEmptyBytes.into());
                    }
                }
                break;
            }

            shift += 7;
            if shift >= bits {
                return Err(Error::TooLong(DecodeState {
                    cur,
                    shift: shift - 7,
                    byte,
                }));
            }

            last_byte = byte;
            byte = self.byte()?;
            first = false;
        }

        if shift > bits - 7 {
            // extra bits mask
            let mask = !((1 << (bits - shift - 1)) - 1);
            if byte & 0x40 != 0 {
                if shift > bits - 7 && !(mask & byte | 0x80 | !mask) != 0 {
                    return Err(Error::TooLong(DecodeState {
                        cur,
                        shift,
                        byte,
                    }));
                }
            } else {
                if shift > bits - 7 && mask & byte != 0 {
                    return Err(Error::TooLong(DecodeState {
                        cur,
                        shift,
                        byte,
                    }));
                }
            }
        }

        let mut res = T::from_unsigned(cur);
        if shift < bits - 7 && byte & 0x40 != 0 {
            res.one_fill_left(shift + 7);
        }

        Ok(res)
    }

    pub(crate) fn sleb128<N: NumSigned>(&mut self) -> crate::Result<N> {
        let res = self.sleb128_inner::<N>(None);
        match res {
            Ok(n) => Ok(n),
            Err(Error::Others(err)) => Err(err),
            Err(Error::TooLong(state)) => {
                let res2 = self.sleb128_inner::<i128>(Some(state.into_128()));

                match res2 {
                    Ok(n) => Err(crate::Error::SLEB128LongerThanTargetType(n, core::any::type_name::<N>())),
                    Err(Error::Others(err)) => Err(err),
                    Err(Error::TooLong { .. }) => Err(crate::Error::LEB128LongerThan128)
                }
            }
        }
    }
}

// endregion

// region: writer

use crate::{byte_storage::Output, writer::Writer};

impl<O: Output> Writer<O> {
    pub(crate) fn uleb128(&mut self, mut n: impl NumUnsigned) {
        loop {
            let byte = n.trunc_u8() & 0x7F;
            n.shr_assign(7);

            if n.all_zero() {
                self.byte(byte);
                break;
            } else {
                self.byte(byte | 0x80);
            }
        }
    }

    pub(crate) fn sleb128(&mut self, n: impl NumSigned) {
        let mut n = n.as_unsigned();
        loop {
            let byte = n.trunc_u8() & 0x7F;
            n.sar_assign(7);

            let sign = byte & 0x40;
            if (n.all_zero() && sign == 0) || (n.all_one() && sign != 0)
            {
                self.byte(byte);
                break;
            } else {
                self.byte(byte | 0x80);
            }
        }
    }
}

// endregion
