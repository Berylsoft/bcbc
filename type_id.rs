use crate::*;

impl TypeId {
    pub fn encode<B: AsRef<[u8]> + ByteStorage>(&self) -> Value<B> {
        let tag = Value::Uint(self.as_type_id_tag() as u8 as u128);
        let value = match self {
            TypeId::Anonymous => Value::Tuple(Box::new([])),
            TypeId::Std { schema, id } => Value::Tuple(Box::new([
                Value::Uint(*schema),
                Value::Uint(*id),
            ]))
        };
        Value::Tuple(Box::new([tag, value]))
    }
}
