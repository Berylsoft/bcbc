#![allow(dead_code)]

use super::{*, byte_storage::Input, leb128::*};

// TODO limit recursive levels?

// We can't avoid allocs completely because of nested values and indefinite-length sequences.
// So we should check for allocation at sequence creates to ensure no panic.
#[inline]
fn alloc_seq<T, F: FnMut(()) -> Result<T>>(len: usize, f: F) -> Result<Box<[T]>> {
    core::iter::repeat_n((), len).map(f).collect()
}

enum OptionWithType<T> {
    None(Type),
    Some(T),
}

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
    fn bytes(&mut self, len: usize) -> Result<B> {
        Ok(self.inner.bytes(len)?)
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

    fn tag(&mut self) -> Result<Tag> {
        self.byte()?.try_into()
    }

    fn exp_tag(&mut self, exp_tag: Tag) -> Result<()> {
        let tag = self.tag()?;
        if tag == exp_tag {
            Ok(())
        } else {
            Err(Error::ExpectedTypeMismatch(tag))
        }
    }

    fn h_fixed_tuple_like(&mut self, exp_len: u8) -> Result<()> {
        let len: u8 = self.uleb128()?;
        if len == exp_len {
            Ok(())
        } else {
            Err(Error::FixedTupleLen(len))
        }
    }
    
    fn fh_fixed_tuple(&mut self, exp_len: u8) -> Result<()> {
        self.exp_tag(Tag::Tuple)?;
        self.h_fixed_tuple_like(exp_len)?;
        Ok(())
    }


    fn c_uint<N: NumUnsigned>(&mut self) -> Result<N> {
        self.uleb128()
    }

    fn v_uint<N: NumUnsigned>(&mut self) -> Result<N> {
        self.exp_tag(Tag::Uint)?;
        self.c_uint()
    }


    fn c_int<N: NumSigned>(&mut self) -> Result<N> {
        self.sleb128()
    }

    fn v_int<N: NumSigned>(&mut self) -> Result<N> {
        self.exp_tag(Tag::Int)?;
        self.c_int()
    }


    fn c_bool(&mut self) -> Result<bool> {
        let n: u8 = self.uleb128()?;
        match n {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(Error::U8ToBool(n)),
        }
    }

    fn v_bool(&mut self) -> Result<bool> {
        self.exp_tag(Tag::Bool)?;
        self.c_bool()
    }


    fn h_uints(&mut self) -> Result<usize> {
        let len: usize = self.uleb128()?;
        if len > self.max_lens.uints {
            return Err(Error::MaxLen(MaxLenType::Uints, MaxLenExceedValue::Size(len)))
        }
        Ok(len)
    }

    fn fh_uints(&mut self) -> Result<usize> {
        self.exp_tag(Tag::Uints)?;
        self.h_uints()
    }

    #[inline]
    fn i_uints(&mut self) -> Result<u128> {
        self.uleb128()
    }

    #[inline]
    fn ic_uints(&mut self, len: usize) -> Result<Box<[u128]>> {
        alloc_seq(len, |_| self.i_uints())
    }

    fn c_uints(&mut self) -> Result<Box<[u128]>> {
        let len = self.h_uints()?;
        self.ic_uints(len)
    }

    fn v_uints(&mut self) -> Result<Box<[u128]>> {
        let len = self.fh_uints()?;
        self.ic_uints(len)
    }


    fn c_bytes(&mut self) -> Result<B> {
        let len: usize = self.uleb128()?;
        if len > self.max_lens.bytes {
            return Err(Error::MaxLen(MaxLenType::Bytes, MaxLenExceedValue::Size(len)))
        }
        self.bytes(len)
    }

    fn v_bytes(&mut self) -> Result<B> {
        self.exp_tag(Tag::Bytes)?;
        self.c_bytes()
    }


    fn h_string(&mut self) -> Result<usize> {
        let len: usize = self.uleb128()?;
        if len > self.max_lens.string {
            return Err(Error::MaxLen(MaxLenType::String, MaxLenExceedValue::Size(len)))
        }
        Ok(len)
    }

    fn fh_string(&mut self) -> Result<usize> {
        self.exp_tag(Tag::String)?;
        self.h_string()
    }

    #[inline]
    fn i_string(&mut self) -> Result<char> {
        let c: u32 = self.uleb128()?;
        char::from_u32(c).ok_or(Error::U32ToChar(c))
    }

    #[inline]
    fn ic_string(&mut self, len: usize) -> Result<Box<[char]>> {
        alloc_seq(len, |_| self.i_string())
    }

    fn c_string(&mut self) -> Result<Box<[char]>> {
        let len = self.h_string()?;
        self.ic_string(len)
    }

    fn v_string(&mut self) -> Result<Box<[char]>> {
        let len = self.fh_string()?;
        self.ic_string(len)
    }


    fn h_tuple(&mut self) -> Result<usize> {
        let len: usize = self.uleb128()?;
        if len > self.max_lens.tuple {
            return Err(Error::MaxLen(MaxLenType::Tuple, MaxLenExceedValue::Size(len)))
        }
        Ok(len)
    }

    fn fh_tuple(&mut self) -> Result<usize> {
        self.exp_tag(Tag::Tuple)?;
        self.h_tuple()
    }

    #[inline]
    fn i_tuple(&mut self) -> Result<Value<B>> {
        self.value()
    }

    #[inline]
    fn ic_tuple(&mut self, len: usize) -> Result<Box<[Value<B>]>> {
        alloc_seq(len, |_| self.i_tuple())
    }

    fn c_tuple(&mut self) -> Result<Box<[Value<B>]>> {
        let len = self.h_tuple()?;
        self.ic_tuple(len)
    }

    fn v_tuple(&mut self) -> Result<Box<[Value<B>]>> {
        let len = self.fh_tuple()?;
        self.ic_tuple(len)
    }


    fn h_list_items(&mut self) -> Result<usize> {
        let len: usize = self.uleb128()?;
        if len > self.max_lens.list {
            return Err(Error::MaxLen(MaxLenType::List, MaxLenExceedValue::Size(len)))
        }
        Ok(len)
    }

    fn fh_list_items(&mut self) -> Result<usize> {
        self.exp_tag(Tag::ListItems)?;
        self.h_list_items()
    }


    // TODO only c_ & v_ ?

    fn h_generics(&mut self) -> Result<usize> {
        let len: usize = self.uleb128()?;
        if len > self.max_lens.generics {
            return Err(Error::MaxLen(MaxLenType::Generics, MaxLenExceedValue::Size(len)))
        }
        Ok(len)
    }

    fn fh_generics(&mut self) -> Result<usize> {
        self.exp_tag(Tag::Generics)?;
        self.h_generics()
    }

    #[inline]
    fn i_generics(&mut self) -> Result<Type> {
        self.v_type()
    }

    #[inline]
    fn ic_generics(&mut self, len: usize) -> Result<Box<[Type]>> {
        alloc_seq(len, |_| self.i_generics())
    }

    fn c_generics(&mut self) -> Result<Box<[Type]>> {
        let len = self.h_generics()?;
        self.ic_generics(len)
    }

    fn v_generics(&mut self) -> Result<Box<[Type]>> {
        let len = self.fh_generics()?;
        self.ic_generics(len)
    }


    fn h_list(&mut self) -> Result<OptionWithType<usize>> {
        self.h_fixed_tuple_like(2)?;
        let is_some = self.v_bool()?;
        Ok(if is_some {
            let len = self.fh_list_items()?;
            if len == 0 {
                return Err(Error::EmptyListInNotEmptyMark);
            }
            OptionWithType::Some(len)
        } else {
            OptionWithType::None(self.v_type()?)
        })
    }

    fn fh_list(&mut self) -> Result<OptionWithType<usize>> {
        self.exp_tag(Tag::List)?;
        self.h_list()
    }

    #[inline]
    fn i_list(&mut self) -> Result<Value<B>> {
        self.value()
    }

    #[inline]
    fn ic_list(&mut self, len: usize) -> Result<Box<[Value<B>]>> {
        alloc_seq(len, |_| self.i_list())
    }

    fn p_list(&mut self, prim: OptionWithType<usize>) -> Result<(Type, Box<[Value<B>]>)> {
        Ok(match prim {
            OptionWithType::None(r#type) => {
                (r#type, Box::new([]))
            }
            OptionWithType::Some(len) => {
                let items = self.ic_list(len)?;
                let r#type = items.iter().next()
                    .ok_or(Error::EmptyListInNotEmptyMark)?.as_type();
                (r#type, items)
            }
        })
    }

    fn c_list(&mut self) -> Result<(Type, Box<[Value<B>]>)> {
        let prim = self.h_list()?;
        self.p_list(prim)
    }

    fn v_list(&mut self) -> Result<(Type, Box<[Value<B>]>)> {
        let prim = self.fh_list()?;
        self.p_list(prim)
    }


    fn h_option(&mut self) -> Result<OptionWithType<()>> {
        self.h_fixed_tuple_like(2)?;
        let is_some = self.v_bool()?;
        Ok(if is_some {
            OptionWithType::Some(())
        } else {
            OptionWithType::None(self.v_type()?)
        })
    }

    fn fh_option(&mut self) -> Result<OptionWithType<()>> {
        self.exp_tag(Tag::Option)?;
        self.h_option()
    }

    fn p_option(&mut self, prim: OptionWithType<()>) -> Result<(Type, Option<Value<B>>)> {
        Ok(match prim {
            OptionWithType::None(r#type) => {
                (r#type, None)
            }
            OptionWithType::Some(()) => {
                let val = self.value()?;
                let r#type = val.as_type();
                (r#type, Some(val))
            }
        })
    }

    fn c_option(&mut self) -> Result<(Type, Option<Value<B>>)> {
        let prim = self.h_option()?;
        self.p_option(prim)
    }

    fn v_option(&mut self) -> Result<(Type, Option<Value<B>>)> {
        let prim = self.fh_option()?;
        self.p_option(prim)
    }


    fn h_alias(&mut self) -> Result<(TypeId, Box<[Type]>)> {
        self.h_fixed_tuple_like(3)?;
        let type_id = self.v_type_id()?;
        let generics = self.v_generics()?;
        Ok((type_id, generics))
    }

    fn fh_alias(&mut self) -> Result<(TypeId, Box<[Type]>)> {
        self.exp_tag(Tag::Alias)?;
        self.h_alias()
    }

    fn c_alias(&mut self) -> Result<(TypeId, Box<[Type]>, Value<B>)> {
        let (type_id, generics) = self.h_alias()?;
        let value = self.value()?;
        Ok((type_id, generics, value))
    }

    fn v_alias(&mut self) -> Result<(TypeId, Box<[Type]>, Value<B>)> {
        let (type_id, generics) = self.fh_alias()?;
        let value = self.value()?;
        Ok((type_id, generics, value))
    }


    fn c_enum(&mut self) -> Result<(TypeId, VariantId)> {
        self.h_fixed_tuple_like(2)?;
        let type_id = self.v_type_id()?;
        let variant_id: VariantId = self.v_uint()?;
        if variant_id > self.max_lens.variants {
            return Err(Error::MaxLen(MaxLenType::Variants, MaxLenExceedValue::Id(variant_id)))
        }
        Ok((type_id, variant_id))
    }

    fn v_enum(&mut self) -> Result<(TypeId, VariantId)> {
        self.exp_tag(Tag::Enum)?;
        self.c_enum()
    }


    fn h_choice(&mut self) -> Result<(TypeId, Box<[Type]>, VariantId)> {
        self.h_fixed_tuple_like(4)?;
        let type_id = self.v_type_id()?;
        let generics = self.v_generics()?;
        let variant_id: VariantId = self.v_uint()?;
        if variant_id > self.max_lens.variants {
            return Err(Error::MaxLen(MaxLenType::Variants, MaxLenExceedValue::Id(variant_id)))
        }
        Ok((type_id, generics, variant_id))
    }

    fn fh_choice(&mut self) -> Result<(TypeId, Box<[Type]>, VariantId)> {
        self.exp_tag(Tag::Choice)?;
        self.h_choice()
    }

    #[allow(clippy::type_complexity)]
    fn c_choice(&mut self) -> Result<(TypeId, Box<[Type]>, VariantId, Value<B>)> {
        let (type_id, generics, variant_id) = self.h_choice()?;
        let value = self.value()?;
        Ok((type_id, generics, variant_id, value))
    }

    #[allow(clippy::type_complexity)]
    fn v_choice(&mut self) -> Result<(TypeId, Box<[Type]>, VariantId, Value<B>)> {
        let (type_id, generics, variant_id) = self.fh_choice()?;
        let value = self.value()?;
        Ok((type_id, generics, variant_id, value))
    }


    fn h_struct(&mut self) -> Result<(TypeId, Box<[Type]>, usize)> {
        self.h_fixed_tuple_like(3)?;
        let type_id = self.v_type_id()?;
        let generics = self.v_generics()?;
        let len = self.fh_tuple()?;
        Ok((type_id, generics, len))
    }

    fn fh_struct(&mut self) -> Result<(TypeId, Box<[Type]>, usize)> {
        self.exp_tag(Tag::Struct)?;
        self.h_struct()
    }

    fn i_struct(&mut self) -> Result<Value<B>> {
        self.value()
    }

    fn ic_struct(&mut self, len: usize) -> Result<Box<[Value<B>]>> {
        alloc_seq(len, |_| self.i_struct())
    }

    #[allow(clippy::type_complexity)]
    fn c_struct(&mut self) -> Result<(TypeId, Box<[Type]>, Box<[Value<B>]>)> {
        let (type_id, generics, len) = self.h_struct()?;
        let values = self.ic_struct(len)?;
        Ok((type_id, generics, values))
    }

    #[allow(clippy::type_complexity)]
    fn v_struct(&mut self) -> Result<(TypeId, Box<[Type]>, Box<[Value<B>]>)> {
        let (type_id, generics, len) = self.fh_struct()?;
        let values = self.ic_struct(len)?;
        Ok((type_id, generics, values))
    }


    fn c_type(&mut self) -> Result<Type> {
        self.h_fixed_tuple_like(2)?;
        let type_tag: TypeTag = self.v_uint::<u8>()?.try_into()?;

        macro_rules! type_tag_to_type_impl {
            (
                direct {$($direct_name:ident)*}
                type {$($type_name:ident)*}
                type_id {$($type_id_name:ident)*}
                type_id generics {$($type_id_generics_name:ident)*}
                $($tt:tt)*
            ) => {
                match type_tag {
                    $(TypeTag::$direct_name => {
                        self.fh_fixed_tuple(0)?;
                        Type::$direct_name
                    })*
                    $($tt)*
                    $(TypeTag::$type_name => {
                        let type2 = self.v_type()?;
                        Type::$type_name(Box::new(type2))
                    })*
                    $(TypeTag::$type_id_name => {
                        let type_id = self.v_type_id()?;
                        Type::$type_id_name(type_id)
                    },)*
                    $(TypeTag::$type_id_generics_name => {
                        self.fh_fixed_tuple(2)?;
                        let type_id = self.v_type_id()?;
                        let generics = self.v_generics()?;
                        Type::$type_id_generics_name(type_id, generics)
                    })*
                }
            };
        }

        Ok(type_tag_to_type_impl! {
            direct {
                Unknown
                Uint
                Int
                Bool
                Uints
                Bytes
                String
                Type
                TypeId
            }
            type {
                List
                Option
            }
            type_id {
                Enum
            }
            type_id generics {
                Alias
                Choice
                Struct
            }
            TypeTag::Tuple => {
                let len = self.fh_tuple()?;
                if len > self.max_lens.tuple {
                    return Err(Error::MaxLen(MaxLenType::Tuple, MaxLenExceedValue::Size(len)))
                }
                let value_types = alloc_seq(len, |_| self.v_type())?;
                Type::Tuple(value_types)
            }
        })
    }

    fn v_type(&mut self) -> Result<Type> {
        self.exp_tag(Tag::Type)?;
        self.c_type()
    }


    fn c_type_id(&mut self) -> Result<TypeId> {
        self.h_fixed_tuple_like(2)?;
        let type_id_tag: TypeIdTag = self.v_uint::<u8>()?.try_into()?;
        Ok(match type_id_tag {
            TypeIdTag::Anonymous => {
                self.fh_fixed_tuple(0)?;
                TypeId::Anonymous
            }
            TypeIdTag::Std => {
                let id: u128 = self.v_uint()?;
                TypeId::Std(id)
            }
        })
    }

    fn v_type_id(&mut self) -> Result<TypeId> {
        self.exp_tag(Tag::TypeId)?;
        self.c_type_id()
    }


    pub fn value(&mut self) -> Result<Value<B>> {
        let tag = self.tag()?;
        Ok(match tag {
            Tag::Uint => Value::Uint(self.c_uint()?),
            Tag::Int => Value::Int(self.c_int()?),
            Tag::Bool => Value::Bool(self.c_bool()?),
            Tag::Uints => Value::Uints(self.c_uints()?),
            Tag::Bytes => Value::Bytes(self.c_bytes()?),
            Tag::String => Value::String(self.c_string()?),
            Tag::Tuple => Value::Tuple(self.c_tuple()?),
            Tag::List => {
                let (r#type, values) = self.c_list()?;
                Value::List(r#type, values)
            }
            Tag::Option => {
                let (r#type, value) = self.c_option()?;
                Value::Option(r#type, value.map(Box::new))
            }
            Tag::Alias => {
                let (type_id, generics, value) = self.c_alias()?;
                Value::Alias(type_id, generics, Box::new(value))
            }
            Tag::Enum => {
                let (type_id, variant_id) = self.c_enum()?;
                Value::Enum(type_id, variant_id)
            }
            Tag::Choice => {
                let (type_id, generics, variant_id, value) = self.c_choice()?;
                Value::Choice(type_id, generics, variant_id, Box::new(value))
            }
            Tag::Struct => {
                let (type_id, generics, values) = self.c_struct()?;
                Value::Struct(type_id, generics, values)
            },
            Tag::Type => Value::Type(self.c_type()?),
            Tag::TypeId => Value::TypeId(self.c_type_id()?),
            Tag::ListItems
            | Tag::Generics => {
                return Err(Error::ImplicitTypeOnTop);
            }
        })
    }
}

// TODO default max lens
impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn decode_with_max_lens<I: Input<Storage = B>>(buf: B, max_lens: MaxLens) -> FullResult<Value<B>, B> {
        let mut reader = Reader::<I>::new(buf, max_lens);
        let val = reader.value();
        reader.finish_with(val)
    }

    pub fn decode<I: Input<Storage = B>>(buf: B) -> FullResult<Value<B>, B> {
        Self::decode_with_max_lens::<I>(buf, DEFAULT_MAX_LENS)
    }

    // cannot return FullResult
    pub fn decode_first_value_with_max_lens<I: Input<Storage = B>>(buf: B, max_lens: MaxLens) -> (Result<Value<B>>, B) {
        let mut reader = Reader::<I>::new(buf, max_lens);
        let res = reader.value();
        (res, reader.into_rest().leak())
    }

    // cannot return FullResult
    pub fn decode_first_value<I: Input<Storage = B>>(buf: B) -> (Result<Value<B>>, B) {
        Self::decode_first_value_with_max_lens::<I>(buf, DEFAULT_MAX_LENS)
    }
}
