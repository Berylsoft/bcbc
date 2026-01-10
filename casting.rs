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
            Uints
            String
            Bytes
            List
            Tuple
            Alias
            Enum
            Choice
            Struct
            Type
            TypeId
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
                Uints
                String
                Bytes
                Type
                TypeId
            }
            non_empty {
                List
                Tuple
                Alias
                Enum
                Choice
                Struct
            }
        }
    }
}

impl TypeId {
    pub fn as_type_id_tag(&self) -> TypeIdTag {
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
