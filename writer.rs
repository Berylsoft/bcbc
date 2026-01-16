use super::*;

// TODO writer error?

struct Writer<O> {
    output: O,
}

impl<O: Output> Writer<O> {
    // begin wrapper impls

    fn new() -> Writer<O> {
        Writer { output: Default::default() }
    }

    fn into_inner(self) -> O::Storage {
        self.output.leak()
    }

    #[inline(always)]
    fn bytes<B: AsRef<[u8]> + ByteStorage>(&mut self, bytes: B) {
        self.output.bytes(bytes);
    }

    #[inline(always)]
    fn byte(&mut self, byte: u8) {
        self.output.byte(byte);
    }

    // end wrapper impls

    // https://github.com/BillGoldenWater/playground/blob/eb898e9/rust/leb128/src/lib.rs
    // TODO: byte-storage extension?

    fn uleb128(&mut self, mut n: u128) {
        loop {
            let byte = n as u8 & 0x7F;
            n >>= 7;

            if n == 0 {
                self.byte(byte);
                break;
            } else {
                self.byte(byte | 0x80);
            }
        }
    }

    fn sleb128(&mut self, mut n: i128) {
        loop {
            let byte = n as u8 & 0x7F;
            n >>= 7;

            let sign = byte & 0x40;
            if (n == 0 && sign == 0) || (n == -1 && sign != 0) {
                self.byte(byte);
                break;
            } else {
                self.byte(byte | 0x80);
            }
        }
    }

    fn tag(&mut self, tag: Tag) {
        self.byte(tag as u8);
    }

    fn v_uint(&mut self, n: u128) {
        self.tag(Tag::Uint);
        self.uleb128(n);
    }

    fn v_int(&mut self, n: i128) {
        self.tag(Tag::Int);
        self.sleb128(n);
    }

    fn v_uints(&mut self, uints: &[u128]) {
        self.tag(Tag::Uints);
        self.uleb128(uints.len() as u128);
        for n in uints {
            self.uleb128(*n);
        }
    }

    fn v_bytes<B: AsRef<[u8]> + ByteStorage>(&mut self, bytes: B) {
        self.tag(Tag::Bytes);
        self.uleb128(bytes.as_ref().len() as u128);
        self.bytes(bytes);
    }

    fn v_string(&mut self, chars: &[char]) {
        self.tag(Tag::String);
        self.uleb128(chars.len() as u128);
        for char in chars {
            self.uleb128(*char as u128);
        }
    }

    fn h_tuple_need_values(&mut self, len: u128) {
        self.tag(Tag::Tuple);
        self.uleb128(len);
    }

    fn h_tuple_like_need_values(&mut self, tag: Tag, len: u128) {
        self.tag(tag);
        self.uleb128(len);
    }

    fn v_type_id(&mut self, type_id: &TypeId) {
        self.h_tuple_like_need_values(Tag::TypeId, 2);
        self.v_uint(type_id.as_type_id_tag() as u8 as u128);
        match type_id {
            TypeId::Anonymous => {
                self.h_tuple_need_values(0);
            }
            TypeId::Std { schema, id } => {
                self.h_tuple_need_values(2);
                self.v_uint(*schema);
                self.v_uint(*id);
            }
        };
    }

    fn h_list_need_values(&mut self, r#type: &Type, len: u128) {
        self.h_tuple_like_need_values(Tag::List, 2);
        self.v_type(r#type);
        self.h_tuple_need_values(len);
    }

    fn v_type(&mut self, r#type: &Type) {
        self.h_tuple_like_need_values(Tag::Type, 2);
        self.v_uint(r#type.as_type_tag() as u8 as u128);
        match r#type {
            Type::Unknown
            | Type::Uint
            | Type::Int
            | Type::Uints
            | Type::Bytes
            | Type::String
            | Type::Type
            | Type::TypeId => {
                self.h_tuple_need_values(0);
            }

            Type::List(type2) => {
                self.v_type(type2);
            }

            Type::Tuple(value_types) => {
                // if uses list here, h_list and v_type refer to each other. may causes dead loop?
                self.h_list_need_values(&Type::Type, value_types.len() as u128);
                for value_type in value_types {
                    self.v_type(value_type);
                }
            }

            Type::Alias(type_id)
            | Type::Enum(type_id)
            | Type::Choice(type_id)
            | Type::Struct(type_id) => {
                self.v_type_id(type_id);
            }
        }
    }

    fn v_generics(&mut self, generics: &[Type]) {
        self.h_list_need_values(&Type::Type, generics.len() as u128);
        for generic in generics {
            self.v_type(generic);
        }
    }

    fn h_alias_need_value(&mut self, type_id: &TypeId, generics: &[Type]) {
        self.h_tuple_like_need_values(Tag::Alias, 3);
        self.v_type_id(type_id);
        self.v_generics(generics);
    }

    fn v_enum(&mut self, type_id: &TypeId, generics: &[Type], var_id: VariantId) {
        self.h_tuple_like_need_values(Tag::Enum, 3);
        self.v_type_id(type_id);
        self.v_generics(generics);
        self.v_uint(var_id);
    }

    fn h_choice_need_value(&mut self, type_id: &TypeId, generics: &[Type], var_id: VariantId) {
        self.h_tuple_like_need_values(Tag::Choice, 4);
        self.v_type_id(type_id);
        self.v_generics(generics);
        self.v_uint(var_id);
    }

    fn h_struct_need_values(&mut self, type_id: &TypeId, generics: &[Type], len: u128) {
        self.h_tuple_like_need_values(Tag::Struct, 3);
        self.v_type_id(type_id);
        self.v_generics(generics);
        self.h_tuple_need_values(len);
    }
}

impl<O: Output> Writer<O> {
    fn v_tuple<B: AsRef<[u8]> + ByteStorage>(&mut self, values: &[Value<B>]) {
        self.h_tuple_need_values(values.len() as u128);
        for value in values {
            self.value(value);
        }
    }

    fn v_list<B: AsRef<[u8]> + ByteStorage>(&mut self, r#type: &Type, values: &[Value<B>]) {
        self.h_list_need_values(r#type, values.len() as u128);
        for value in values {
            self.value(value);
        }
    }

    fn v_alias<B: AsRef<[u8]> + ByteStorage>(&mut self, type_id: &TypeId, generics: &[Type], value: &Value<B>) {
        self.h_alias_need_value(type_id, generics);
        self.value(value);
    }

    fn v_choice<B: AsRef<[u8]> + ByteStorage>(&mut self, type_id: &TypeId, generics: &[Type], var_id: VariantId, value: &Value<B>) {
        self.h_choice_need_value(type_id, generics, var_id);
        self.value(value);
    }

    fn v_struct<B: AsRef<[u8]> + ByteStorage>(&mut self, type_id: &TypeId, generics: &[Type], values: &[Value<B>]) {
        self.h_struct_need_values(type_id, generics, values.len() as u128);
        for value in values {
            self.value(value);
        }
    }

    fn value<B: AsRef<[u8]> + ByteStorage>(&mut self, value: &Value<B>) {
        match value {
            Value::Uint(n) => self.v_uint(*n),
            Value::Int(n) => self.v_int(*n),
            Value::Uints(uints) => self.v_uints(uints),
            Value::Bytes(bytes) => self.v_bytes(bytes),
            Value::String(chars) => self.v_string(chars),
            Value::List(r#type, values) => self.v_list(r#type, values),
            Value::Tuple(values) => self.v_tuple(values),
            Value::Alias(type_id, generics, value) => self.v_alias(type_id, generics, value),
            Value::Enum(type_id, generics, var_id) => self.v_enum(type_id, generics, *var_id),
            Value::Choice(type_id, generics, var_id, value) => self.v_choice(type_id, generics, *var_id, value),
            Value::Struct(type_id, generics, values) => self.v_struct(type_id, generics, values),
            Value::Type(r#type) => self.v_type(r#type),
            Value::TypeId(type_id) => self.v_type_id(type_id),
        }
    }
}

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn encode<O: Output>(&self) -> O::Storage {
        let mut writer = Writer::<O>::new();
        writer.value(self);
        writer.into_inner()
    }
}
