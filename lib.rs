#![deny(unused_results)]
#![forbid(unsafe_code)]

#![no_std]

extern crate alloc;
use alloc::boxed::Box;

use foundations::{error_enum, num_enum_reverse};
pub use byte_storage;
use byte_storage::*;

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

    Alias(TypeId),
    Enum(TypeId),
    Choice(TypeId),
    Struct(TypeId),

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
    Enum(TypeId, Box<[Type]>, VariantId),
    Choice(TypeId, Box<[Type]>, VariantId, Box<Value<B>>),
    Struct(TypeId, Box<[Type]>, Box<[Value<B>]>),

    Type(Type),
    TypeId(TypeId),
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
        // TODO temp solution
        TooLongLen(usize),
        Tag(u8),
        TypeTag(u8),
        TypeIdTag(u8),
        LEB128TooLong,
    } convert {
        Read => ReadError,
        Fatal => Fatal,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fatal {
}

type Result<T> = core::result::Result<T, Error>;
type FullResult<T, B> = core::result::Result<T, FullError<B>>;
type FatalResult<T> = core::result::Result<T, Fatal>;

pub(crate) mod leb128_num_traits;
pub mod casting;

pub mod reader;
pub mod writer;

#[cfg(test)]
mod tests;
