use super::*;

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub const fn as_tag(&self) -> Tag {
        macro_rules! as_tag_impl {
            (
                $($name:ident)*
            ) => {
                match self {
                    $(Value::$name(..) => Tag::$name,)*
                }
            };
        }

        as_tag_impl! {
            Uint
            Int
            Bool
            Uints
            String
            Bytes
            Tuple
            List
            Option
            Alias
            Enum
            Choice
            Struct
            Type
            TypeId
        }
    }

    fn as_type(&self) -> Type {
        macro_rules! as_type_impl {
            (
                direct {$($direct_name:ident)*}
                type {$($type_name:ident)*}
                type_id {$($type_id_name:ident)*}
                $($tt:tt)*
            ) => {
                match self {
                    $(Value::$direct_name(..) => Type::$direct_name,)*
                    $($tt)*
                    $(Value::$type_name(r#type, ..) => Type::$type_name(Box::new(r#type.clone())),)*
                    $(Value::$type_id_name(type_id, ..) => Type::$type_id_name(type_id.clone()),)*
                }
            };
        }

        as_type_impl! {
            direct {
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
                Alias
                Enum
                Choice
                Struct
            }
            Value::Tuple(values) => {
                Type::Tuple(values.iter().map(|value| value.as_type()).collect())
            }
        }
    }
}

impl Type {
    pub const fn as_type_tag(&self) -> TypeTag {
        macro_rules! as_tag_impl {
            (
                empty {$($empty_name:ident)*}
                non_empty {$($name:ident)*}
            ) => {
                match self {
                    $(Type::$empty_name => TypeTag::$empty_name,)*
                    $(Type::$name(..) => TypeTag::$name,)*
                }
            };
        }

        as_tag_impl! {
            empty {
                Unknown
                Uint
                Int
                Bool
                Uints
                String
                Bytes
                Type
                TypeId
            }
            non_empty {
                Tuple
                List
                Option
                Alias
                Enum
                Choice
                Struct
            }
        }
    }
}

impl TypeId {
    pub const fn as_type_id_tag(&self) -> TypeIdTag {
        macro_rules! as_tag_impl {
            (
                empty {$($empty_name:ident)*}
                non_empty {$($name:ident)*}
            ) => {
                match self {
                    $(TypeId::$empty_name => TypeIdTag::$empty_name,)*
                    $(TypeId::$name { .. } => TypeIdTag::$name,)*
                }
            };
        }

        as_tag_impl! {
            empty {
                Anonymous
            }
            non_empty {
                Std
            }
        }
    }
}
