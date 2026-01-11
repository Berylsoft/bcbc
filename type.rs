use crate::*;

impl Type {
    pub fn encode<B: AsRef<[u8]> + ByteStorage>(&self) -> Value<B> {
        let tag = Value::Uint(self.as_type_tag() as u8 as u128);
        let value = match self {
            Type::Unknown
            | Type::Uint
            | Type::Int
            | Type::Uints
            | Type::Bytes
            | Type::String
            | Type::Type
            | Type::TypeId => Value::Tuple(Box::new([])),

            Type::List(ty) => Value::Type(*ty.clone()),

            Type::Tuple(values) => Value::Tuple(
                values.iter().cloned().map(Value::Type).collect()
            ),

            Type::Alias(type_id)
            | Type::Enum(type_id)
            | Type::Choice(type_id)
            | Type::Struct(type_id) => type_id.encode(),
        };
        Value::Tuple(Box::new([tag, value]))
    }
}
