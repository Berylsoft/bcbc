#![deny(unused_results)]
#![forbid(unsafe_code)]

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::boxed::Box;

use foundations::{error_enum, num_enum_reverse};
pub use byte_storage;
use byte_storage::{ByteStorage, ReadError};

type VariantId = u128;

num_enum_reverse! {
    pub enum Tag {
        b'U' = Uint,
        b'I' = Int,
        b'F' = Bool,

        b'N' = Uints,
        b'B' = Bytes,
        b'S' = String,

        b'P' = Tuple,
        b'L' = List,
        // TODO: O&0 distinguish?
        b'O' = Option,

        b'A' = Alias,
        b'E' = Enum,
        b'C' = Choice,
        b'R' = Struct,

        b'T' = Type,
        b'D' = TypeId,

        // implicit tuple types
        b'M' = ListItems,
        b'G' = Generics,
    } as u8 else Error::Tag
}

num_enum_reverse! {
    pub enum TypeTag {
        b'0' = Unknown,

        b'u' = Uint,
        b'i' = Int,
        b'f' = Bool,

        b'n' = Uints,
        b'b' = Bytes,
        b's' = String,

        b'p' = Tuple,
        // TODO: I&l distinguish?
        b'l' = List,
        b'o' = Option,

        b'a' = Alias,
        b'e' = Enum,
        b'c' = Choice,
        b'r' = Struct,

        b't' = Type,
        b'd' = TypeId,
    } as u8 else Error::TypeTag
}

num_enum_reverse! {
    pub enum TypeIdTag {
        b'x' = Anonymous,
        b'y' = Std,
        // b'z' = ThirdParty,
    } as u8 else Error::TypeIdTag
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Unknown,

    Uint,
    Int,
    Bool,

    Uints,
    Bytes,
    String,

    Tuple(Box<[Type]>),
    List(Box<Type>),
    Option(Box<Type>),

    Alias(TypeId, Box<[Type]>),
    Enum(TypeId),
    Choice(TypeId, Box<[Type]>),
    Struct(TypeId, Box<[Type]>),

    Type,
    TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypeId {
    Anonymous,
    Std(u128),
    // TODO third-party
}

// TODO impl<B1: PartialEq<B2>, B2> PartialEq&PartialOrd<Value<B2>> for Value<B1>
// TODO variant structs?
// TODO no value & matching r&w api
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value<B: AsRef<[u8]> + ByteStorage> {
    Uint(u128),
    Int(i128),
    Bool(bool),

    Uints(Box<[u128]>),
    Bytes(B),
    String(Box<[char]>),

    Tuple(Box<[Value<B>]>),
    List(Type, Box<[Value<B>]>),
    Option(Type, Option<Box<Value<B>>>),

    Alias(TypeId, Box<[Type]>, Box<Value<B>>),
    Enum(TypeId, VariantId),
    Choice(TypeId, Box<[Type]>, VariantId, Box<Value<B>>),
    Struct(TypeId, Box<[Type]>, Box<[Value<B>]>),

    Type(Type),
    TypeId(TypeId),
}

pub struct MaxLens {
    pub uints: usize,
    pub bytes: usize,
    pub string: usize,
    pub tuple: usize,
    pub list: usize,
    pub generics: usize,
    pub variants: u128,
}

pub const DEFAULT_MAX_LENS: MaxLens = MaxLens {
    uints: u32::MAX as usize,
    bytes: u32::MAX as usize,
    string: u32::MAX as usize,
    tuple: u32::MAX as usize,
    list: u32::MAX as usize,
    generics: u32::MAX as usize,
    variants: u32::MAX as u128,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaxLenType {
    Uints,
    Bytes,
    String,
    Tuple,
    List,
    Generics,
    Variants,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaxLenExceedValue {
    Size(usize),
    Id(u128),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullError<B> {
    pub err: Error,
    pub buf: B,
    pub pos: usize,
}

error_enum! {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Error {
        Tag(u8),
        TypeTag(u8),
        TypeIdTag(u8),
        U8ToBool(u8),
        LEB128LongerThan128,
        ULEB128LongerThanTargetType(u128, &'static str),
        SLEB128LongerThanTargetType(i128, &'static str),
        LEB128TrailingEmptyBytes,
        MaxLen(MaxLenType, MaxLenExceedValue),
        U32ToChar(u32),
        FixedTupleLen(u8),
        // TODO distinguish inner type & user type
        ExpectedTypeMismatch(Tag),
        EmptyListInNotEmptyMark,
        ImplicitTypeOnTop,
    } convert {
        Read => ReadError,
    }
}

type Result<T> = core::result::Result<T, Error>;
type FullResult<T, B> = core::result::Result<T, FullError<B>>;

pub(crate) mod leb128;
pub mod casting;

pub mod reader;
pub mod writer;

#[cfg(test)]
mod tests;
