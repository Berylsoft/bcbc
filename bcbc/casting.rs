use super::*;

#[inline(always)]
pub const fn bytevar_urange(len: usize) -> core::ops::RangeFrom<usize> {
    (8 - len)..
}

#[inline(always)]
pub const fn bytevar_frange(len: usize) -> core::ops::RangeTo<usize> {
    ..len
}

pub fn bytevar_ulen(buf: &[u8; 8]) -> usize {
    for (i, b) in buf.iter().enumerate() {
        if *b != 0 {
            return 8 - i;
        }
    }
    1
}

pub fn bytevar_flen(buf: &[u8; 8]) -> usize {
    for (i, b) in buf.iter().rev().enumerate() {
        if *b != 0 {
            return 8 - i;
        }
    }
    1
}

#[inline]
pub const fn from_h4l4(h4: H4, l4: L4) -> u8 {
    (h4 as u8) << 4 | (l4 as u8)
}

#[inline]
pub fn to_h4l4(n: u8) -> Result<(H4, L4)> {
    Ok(((n >> 4).try_into()?, (n & 0xf).try_into()?))
}

impl H4 {
    pub const fn is_num(&self) -> bool {
        (*self as u8) < 0x8
    }

    pub const fn to_bytevar_len(self) -> FatalResult<usize> {
        Ok(match self {
            H4::N1 => 1,
            H4::N2 => 2,
            H4::N3 => 3,
            H4::N4 => 4,
            H4::N5 => 5,
            H4::N6 => 6,
            H4::N7 => 7,
            H4::N8 => 8,
            _ => return Err(Fatal::H4ToN(self)),
        })
    }

    pub const fn to_ext1(self) -> FatalResult<Ext1> {
        Ok(match self {
            H4::N1 => Ext1::Unit,
            H4::N2 => Ext1::False,
            H4::N3 => Ext1::True,
            H4::N4 => Ext1::None,
            H4::N5 => Ext1::Some,
            H4::N6 => Ext1::Alias,
            H4::N7 => Ext1::Type,
            H4::N8 => Ext1::TypeId,
            _ => return Err(Fatal::H4ToExt1(self)),
        })
    }

    pub const fn from_bytevar_len(pos: usize) -> FatalResult<H4> {
        Ok(match pos {
            1 => H4::N1,
            2 => H4::N2,
            3 => H4::N3,
            4 => H4::N4,
            5 => H4::N5,
            6 => H4::N6,
            7 => H4::N7,
            8 => H4::N8,
            _ => return Err(Fatal::NToH4(pos)),
        })
    }

    pub const fn from_ext1(ext1: Ext1) -> H4 {
        match ext1 {
            Ext1::Unit   => H4::N1,
            Ext1::False  => H4::N2,
            Ext1::True   => H4::N3,
            Ext1::None   => H4::N4,
            Ext1::Some   => H4::N5,
            Ext1::Alias  => H4::N6,
            Ext1::Type   => H4::N7,
            Ext1::TypeId => H4::N8,
        }
    }
}

impl Type {
    pub const fn as_tag(&self) -> Tag {
        macro_rules! as_tag_impl {
            (
                direct_empty {$($direct_empty_name:ident)*}
                direct {$($direct_name:ident)*}
            ) => {
                match self {
                    $(Type::$direct_empty_name => Tag::$direct_empty_name,)*
                    $(Type::$direct_name(..) => Tag::$direct_name,)*
                }
            };
        }

        as_tag_impl! {
            direct_empty {
                Unknown
                Unit
                Bool
                U8
                U16
                U32
                U64
                I8
                I16
                I32
                I64
                F16
                F32
                F64
                String
                Bytes
                Type
                TypeId
            }
            direct {
                Option
                List
                Map
                Tuple
                Alias
                Enum
                CEnum
                Struct
            }
        }
    }
}

impl Value {
    pub const fn as_tag(&self) -> Tag {
        macro_rules! as_tag_impl {
            (
                direct_empty {$($direct_empty_name:ident)*}
                direct {$($direct_name:ident)*}
            ) => {
                match self {
                    $(Value::$direct_empty_name => Tag::$direct_empty_name,)*
                    $(Value::$direct_name(..) => Tag::$direct_name,)*
                }
            };
        }

        as_tag_impl! {
            direct_empty {
                Unit
            }
            direct {
                Bool
                U8
                U16
                U32
                U64
                I8
                I16
                I32
                I64
                F16
                F32
                F64
                String
                Bytes
                Option
                List
                Map
                Tuple
                Alias
                CEnum
                Enum
                Struct
                Type
                TypeId
            }
        }
    }

    pub fn as_type(&self) -> Type {
        macro_rules! as_type_impl {
            (
                direct_empty {$($direct_empty_name:ident)*}
                direct {$($direct_name:ident)*}
                typeid {$($typeid_name:ident)*}
                $($tt:tt)*
            ) => {
                match self {
                    $(Value::$direct_empty_name => Type::$direct_empty_name,)*
                    $(Value::$direct_name(..) => Type::$direct_name,)*
                    $($tt)*
                    $(Value::$typeid_name(r, ..) => Type::$typeid_name(*r),)*
                }
            };
        }

        as_type_impl! {
            direct_empty {
                Unit
            }
            direct {
                Bool
                U8
                U16
                U32
                U64
                I8
                I16
                I32
                I64
                F16
                F32
                F64
                String
                Bytes
                Type
                TypeId
            }
            typeid {
                Alias
                CEnum
                Enum
                Struct
            }
            Value::Option(t, ..) => Type::Option(Box::new(t.clone())),
            Value::List(t, ..) => Type::List(Box::new(t.clone())),
            Value::Map((tk, tv), ..) => Type::Map(Box::new(tk.clone()), Box::new(tv.clone())),
            Value::Tuple(seq) => Type::Tuple(seq.iter().map(|v| v.as_type()).collect()),
        }
    }
}

impl Value {
    pub fn serialize_from<T: Schema>(val: T) -> Value {
        val.serialize()
    }

    pub fn deserialize_into<T: Schema>(self) -> T {
        T::deserialize(self)
    }
}

// impl Value {
//     pub fn from_float(v: f64) -> Value {
//         Value::Float(v.to_bits())
//     }
// }

macro_rules! into_impl {
    // TODO auto make fn name with concat_ident! and const case convert
    ($($fn_name:ident | $variant:ident)*) => {$(
        pub fn $fn_name(self) {
            if let Value::$variant = self {
                ()
            } else {
                unreachable!()
            }
        }
    )*};
    ($($fn_name:ident -> $ty:ty | $variant:ident)*) => {$(
        pub fn $fn_name(self) -> $ty {
            if let Value::$variant(v) = self {
                v
            } else {
                unreachable!()
            }
        }
    )*};
    ($($fn_name:ident -> $ty:ty | $variant:ident($($val_name:ident$(,)*)*) -> $val_fn:block)*) => {$(
        pub fn $fn_name(self) -> $ty {
            if let Value::$variant($($val_name,)*) = self {
                $val_fn
            } else {
                unreachable!()
            }
        }
    )*};
}

impl Value {
    into_impl! {
        into_unit | Unit
    }

    into_impl! {
        into_bool -> bool      | Bool
        into_u8 -> u8          | U8
        into_u16 -> u16        | U16
        into_u32 -> u32        | U32
        into_u64 -> u64        | U64
        into_i8 -> i8          | I8
        into_i16 -> i16        | I16
        into_i32 -> i32        | I32
        into_i64 -> i64        | I64
        // TODO convert? 
        into_f16 -> u16        | F16
        into_f32 -> u32        | F32
        into_f64 -> u64        | F64
        into_string -> String  | String
        into_bytes -> Vec<u8>  | Bytes
        into_type -> Type      | Type
        into_type_id -> TypeId | TypeId
    }

    into_impl! {
        into_option -> Option<Value>        | Option(_t, v) -> { *v }
        into_list -> Vec<Value>             | List(_t, s) -> { s }
        into_map -> Vec<(Value, Value)>     | Map(_t, s) -> { s }
        into_tuple -> Vec<Value>            | Tuple(s) -> { s }
        into_alias -> Value                 | Alias(_id, v) -> { *v }
        into_c_enum -> EnumVariantId        | CEnum(_id, ev) -> { ev }
        into_enum -> (EnumVariantId, Value) | Enum(_id, ev, v) -> { (ev, *v) }
        into_struct -> Vec<Value>           | Struct(_id, s) -> { s }
    }
}
