#![deny(unused_results)]
#![forbid(unsafe_code)]

#![no_std]

extern crate alloc;
use alloc::boxed::Box;

use foundations::{error_enum, num_enum};
pub use byte_storage;
use byte_storage::*;

pub type EnumVariantId = u64;
pub type TupleItemId = u8;

mod typeid;
pub use typeid::*;

num_enum! {
    pub enum Tag {
        Unknown = 0x00,
        Unit    = 0x01,
        Bool    = 0x02,
        U8      = 0x03,
        U16     = 0x04,
        U32     = 0x05,
        U64     = 0x06,
        I8      = 0x07,
        I16     = 0x08,
        I32     = 0x09,
        I64     = 0x0a,
        F16     = 0x0b,
        F32     = 0x0c,
        F64     = 0x0d,
        String  = 0x0e,
        Bytes   = 0x0f,
        Option  = 0x10,
        List    = 0x11,
        Map     = 0x12,
        Tuple   = 0x13,
        Alias   = 0x14,
        CEnum   = 0x15,
        Enum    = 0x16,
        Struct  = 0x17,
        Type    = 0x18,
        TypeId  = 0x19,
    } as u8 else Error::Tag
}

num_enum! {
    pub enum H4 {
        N1     = 0x0,
        N2     = 0x1,
        N3     = 0x2,
        N4     = 0x3,
        N5     = 0x4,
        N6     = 0x5,
        N7     = 0x6,
        N8     = 0x7,
        String = 0x8,
        Bytes  = 0x9,
        List   = 0xa,
        Map    = 0xb,
        Tuple  = 0xc,
        CEnum  = 0xd,
        Enum   = 0xe,
        Struct = 0xf,
    } as u8 else Fatal::H4
}

num_enum! {
    pub enum L4 {
        U8   = 0x0,
        U16  = 0x1,
        U32  = 0x2,
        U64  = 0x3,
        I8   = 0x4,
        P16  = 0x5,
        P32  = 0x6,
        P64  = 0x7,
        N16  = 0x8,
        N32  = 0x9,
        N64  = 0xa,
        F16  = 0xb,
        F32  = 0xc,
        F64  = 0xd,
        EXT1 = 0xe,
        EXT2 = 0xf,
    } as u8 else Fatal::L4
}

num_enum! {
    pub enum Ext1 {
        Unit   = 0x0,
        False  = 0x1,
        True   = 0x2,
        None   = 0x3,
        Some   = 0x4,
        Alias  = 0x5,
        Type   = 0x6,
        TypeId = 0x7,
    } as u8 else Fatal::Ext1
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Unknown,

    Unit,
    Bool,

    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,

    String,
    Bytes,

    Option(Box<Type>),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Box<[Type]>),
    Alias(TypeId),
    CEnum(TypeId),
    Enum(TypeId),
    Struct(TypeId),

    Type,
    TypeId,
}

// TODO impl<B1: PartialEq<B2>, B2> PartialEq&PartialOrd<Value<B2>> for Value<B1>
// TODO variant structs?
// TODO no value & matching r&w api
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value<B: AsRef<[u8]> + ByteStorage> {
    Unit,
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F16(u16),
    F32(u32),
    F64(u64),

    String(ByteStr<B>),
    Bytes(B),

    Option(Type, Box<Option<Value<B>>>),
    List(Type, Box<[Value<B>]>),
    Map((Type, Type), Box<[(Value<B>, Value<B>)]>),

    Tuple(Box<[Value<B>]>),

    Alias(TypeId, Box<Value<B>>),
    CEnum(TypeId, EnumVariantId),
    Enum(TypeId, EnumVariantId, Box<Value<B>>),
    Struct(TypeId, Box<[Value<B>]>),

    Type(Type),
    TypeId(TypeId),
}

pub const EXT8:  L4 = L4::F32;  // 0xc
pub const EXT16: L4 = L4::F64;  // 0xd
pub const EXT32: L4 = L4::EXT1; // 0xe
pub const EXT64: L4 = L4::EXT2; // 0xf

pub trait Schema {
    const ID: TypeId;
    fn serialize<B: AsRef<[u8]> + ByteStorage>(self) -> Value<B>;
    fn deserialize<B: AsRef<[u8]> + ByteStorage>(val: Value<B>) -> Self;
}

// TODO temp solution
pub const SIZE_MAX: usize = u16::MAX as usize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullError<B> {
    pub err: Error,
    pub buf: B,
    pub pos: usize,
}

error_enum! {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Error {
        // TODO temp solution
        TooLongLen(usize),
        Tag(u8),
        BytevarIntSign { buf: [u8; 8] },
        BytevarLongerThanType { len: usize, nlen: usize, buf: [u8; 8] },
        BytevarLongerThanExpected { len: usize, nlen: usize, exp_len: usize, buf: [u8; 8] },
        BytevarNegZero { buf: [u8; 8] },
        ExtvarTooLong { l4: L4, exp_l4: L4, u: u64 },
        Ext2NotImplemented,
    } convert {
        // Utf8 => { pos: usize, len: usize, error: core::str::Utf8Error },
        Utf8 => core::str::Utf8Error,
        Read => ReadError,
        Fatal => Fatal,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fatal {
    H4(u8),
    L4(u8),
    Ext1(u8),
    H4ToN(H4),
    NToH4(usize),
    H4ToExt1(H4),
    ToSize(u64),
    FromSize(usize),
    // TODO debug vars
    BytevarSlicing,
}

// TODO: use uniform Error like serde_json?
type Result<T> = core::result::Result<T, Error>;
type FullResult<T, B> = core::result::Result<T, FullError<B>>;
type FatalResult<T> = core::result::Result<T, Fatal>;

pub mod casting;
pub mod reader;
pub mod writer;

#[cfg(test)]
mod tests;
