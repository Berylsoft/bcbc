// https://github.com/BillGoldenWater/playground/blob/1799908/rust/leb128/src/lib.rs
// TODO: byte-storage extension?

// region: num traits

pub trait NumUnsigned: Copy {
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

pub trait NumSigned: Copy {
    type UnsignedVariant: NumUnsigned;

    fn as_unsigned(&self) -> Self::UnsignedVariant;
    fn from_unsigned(value: Self::UnsignedVariant) -> Self;
    fn one_fill_left(&mut self, right: u32);
}

macro_rules! impl_num {
    ($ty:ty, $signed_ty:ty) => {
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
        // TODO try_into() on that archs
    }
}

impl_num!(usize, isize);

// endregion

// region: decode state & error

use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LEB128Error<T: NumUnsigned> {
    TooLong(LEB128DecodeState<T>),
    Others(Error),
}

impl<T: NumUnsigned> From<Error> for LEB128Error<T> {
    fn from(err: Error) -> LEB128Error<T> {
        LEB128Error::Others(err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LEB128DecodeState<T: NumUnsigned> {
    pub cur: T,
    pub shift: u32,
    pub byte: u8,
}

impl<T: NumUnsigned> LEB128DecodeState<T> {
    // TODO into_super like orig
    pub fn into_128(self) -> LEB128DecodeState<u128> {
        LEB128DecodeState {
            cur: self.cur.to_u128(),
            shift: self.shift,
            byte: self.byte,
        }
    }
}

impl<T: NumUnsigned> Default for LEB128DecodeState<T> {
    fn default() -> Self {
        Self {
            cur: T::from_u8(0),
            shift: 0,
            byte: 0,
        }
    }
}
