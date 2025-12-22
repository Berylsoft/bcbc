use super::*;

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub const fn as_tag(&self) -> Tag {
        macro_rules! as_tag_impl {
            (
                $($direct_name:ident)*
            ) => {
                match self {
                    $(Value::$direct_name(..) => Tag::$direct_name,)*
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
