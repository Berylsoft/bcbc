use crate::{*, byte_storage::Output, leb128::*};

struct Writer<O> {
    output: O,
    itoa_buffer: itoa::Buffer,
}

impl<O: Output> Writer<O> {
    // begin wrapper impls

    fn new() -> Writer<O> {
        Writer {
            output: Default::default(),
            itoa_buffer: itoa::Buffer::new(),
        }
    }

    fn into_inner(self) -> O::Storage {
        self.output.leak()
    }

    fn utf8_bytes<B: AsRef<[u8]> + ByteStorage>(&mut self, bytes: B) {
        // TODO error
        let _ = core::str::from_utf8(bytes.as_ref()).unwrap();
        self.output.bytes(bytes);
    }

    fn str(&mut self, str: &str) {
        self.output.bytes(str.as_bytes());
    }

    fn ascii_byte(&mut self, byte: u8) {
        // TODO error
        assert!(byte.is_ascii());
        self.output.byte(byte);
    }

    // end wrapper impls

    fn tag(&mut self, tag: Tag) {
        self.ascii_byte(tag as u8);
    }

    fn int_decimal(&mut self, n: impl itoa::Integer) {
        let s = self.itoa_buffer.format(n);
        self.output.bytes(s.as_bytes());
    }

    fn v_uint_tag(&mut self, tag: u8) {
        self.tag(Tag::Uint);
        self.ascii_byte(b'\'');
        self.ascii_byte(tag);
        self.ascii_byte(b'\'');
    }

    fn v_uint_hex(&mut self, n: impl NumUnsigned) {

    }

    fn hex_bytes<B: AsRef<[u8]> + ByteStorage>(&mut self, bytes: B) {
        let hex_len = bytes.as_ref().len() * 2;
        let out_buf = self.output.leak_next_mut(hex_len);
        // TODO error
        hex::encode_to_slice(bytes, out_buf).unwrap()
    }

    fn char(&mut self, char: char) {
        let char_len = char.len_utf8();
        let out_buf = self.output.leak_next_mut(char_len);
        let _ = char.encode_utf8(out_buf);
    }

    fn v_uint(&mut self, n: impl NumUnsigned) {
        self.tag(Tag::Uint);
        self.int_decimal(n);
    }

    fn v_int(&mut self, n: impl NumSigned) {
        self.tag(Tag::Int);
        self.int_decimal(n);
    }

    fn v_bool(&mut self, n: bool) {
        self.tag(Tag::Bool);
        self.int_decimal(n as u8);
    }

    fn v_uints(&mut self, uints: &[impl NumUnsigned]) {
        self.tag(Tag::Uints);
        self.ascii_byte(b'[');
        let mut iter = uints.iter();
        if let Some(first) = iter.next() {
            self.int_decimal(*first);
            for n in iter {
                self.ascii_byte(b' ');
                self.int_decimal(*n);
            }
        }
        self.ascii_byte(b']');
    }

    fn v_bytes<B: AsRef<[u8]> + ByteStorage>(&mut self, bytes: B) {
        self.tag(Tag::Bytes);
        self.ascii_byte(b'"');
        self.hex_bytes(bytes);
        self.ascii_byte(b'"');
    }

    fn v_string(&mut self, chars: &[char]) {
        self.tag(Tag::String);
        self.ascii_byte(b'"');
        let mut utf8_buf = [0; 4];
        for char in chars {
            let encoded = char.encode_utf8(&mut utf8_buf);
            self.str(encoded);
        }
        self.ascii_byte(b'"');
    }

    fn begin_tuple_like<'a>(&'a mut self, tag: Tag) -> TupleWriter<'a, O> {
        self.tag(tag);
        TupleWriter::new(self)
    }

    fn begin_tuple<'a>(&'a mut self) -> TupleWriter<'a, O> {
        self.begin_tuple_like(Tag::Tuple)
    }

    fn v_type_id(&mut self, type_id: &TypeId) {
        let mut tl1 = self.begin_tuple_like(Tag::TypeId);
        {
            tl1.ahead_separator();
            tl1.writer.v_uint_tag(type_id.as_type_id_tag() as u8);
            tl1.ahead_separator();
            match type_id {
                TypeId::Anonymous => {
                    let tl2 = tl1.writer.begin_tuple();
                    tl2.end();
                }
                TypeId::Std(id) => {
                    tl1.writer.v_uint_hex(*id);
                }
            };
        }
        tl1.end();
    }

    fn v_type(&mut self, r#type: &Type) {
        let mut tl1 = self.begin_tuple_like(Tag::Type);
        {
            tl1.ahead_separator();
            tl1.writer.v_uint_tag(r#type.as_type_tag() as u8);
            tl1.ahead_separator();
            match r#type {
                Type::Unknown
                | Type::Uint
                | Type::Int
                | Type::Bool
                | Type::Uints
                | Type::Bytes
                | Type::String
                | Type::Type
                | Type::TypeId => {
                    let tl2 = tl1.writer.begin_tuple();
                    tl2.end();
                }

                Type::Tuple(value_types) => {
                    // if uses list here, h_list and v_type refer to each other. may causes dead loop?
                    let mut tl2 = tl1.writer.begin_tuple();
                    {
                        for value_type in value_types {
                            tl2.ahead_separator();
                            tl2.writer.v_type(value_type);
                        }
                    }
                    tl2.end();
                }

                Type::List(type2)
                | Type::Option(type2) => {
                    tl1.ahead_separator();
                    tl1.writer.v_type(type2);
                }

                Type::Enum(type_id) => {
                    tl1.ahead_separator();
                    tl1.writer.v_type_id(type_id);
                }

                Type::Alias(type_id, generics)
                | Type::Choice(type_id, generics)
                | Type::Struct(type_id, generics) => {
                    let mut tl2 = tl1.writer.begin_tuple();
                    {
                        tl2.ahead_separator();
                        tl2.writer.v_type_id(type_id);
                        tl2.ahead_separator();
                        // tl2.writer.v_generics(generics);
                    }
                    tl2.end();
                }
            }
        }
        tl1.end();
    }
}

struct TupleWriter<'a, O> {
    writer: &'a mut Writer<O>,
    first: bool,
}

impl<'a, O: Output> TupleWriter<'a, O> {
    fn new(writer: &'a mut Writer<O>) -> TupleWriter<'a, O> {
        writer.ascii_byte(b'(');
        TupleWriter { writer, first: true }
    }

    fn ahead_separator(&mut self) {
        if self.first {
            self.first = false;
        } else {
            self.writer.ascii_byte(b' ');
        }
    }

    // fn begin_tuple_like<'b>(&'b mut self) -> TupleWriter<'b, O>
    // where
    //     'a: 'b,
    // {
    //     TupleWriter::new(self.writer)
    // }

    // TODO impl Drop?
    fn end(self) {
        self.writer.ascii_byte(b')');
    }
}

impl<O: Output> Writer<O> {
    fn value<B: AsRef<[u8]> + ByteStorage>(&mut self, value: &Value<B>) {
        match value {
            Value::Uint(n) => self.v_uint(*n),
            Value::Int(n) => self.v_int(*n),
            Value::Bool(n) => self.v_bool(*n),
            Value::Uints(uints) => self.v_uints(uints),
            Value::Bytes(bytes) => self.v_bytes(bytes),
            Value::String(chars) => self.v_string(chars),
            // Value::Tuple(values) => self.v_tuple(values),
            // Value::List(r#type, values) => self.v_list(r#type, values),
            // Value::Option(r#type, value) => self.v_option(r#type, value.as_deref()),
            // Value::Alias(type_id, generics, value) => self.v_alias(type_id, generics, value),
            // Value::Enum(type_id, var_id) => self.v_enum(type_id, *var_id),
            // Value::Choice(type_id, generics, var_id, value) => self.v_choice(type_id, generics, *var_id, value),
            // Value::Struct(type_id, generics, values) => self.v_struct(type_id, generics, values),
            Value::Type(r#type) => self.v_type(r#type),
            Value::TypeId(type_id) => self.v_type_id(type_id),
            _ => unimplemented!()
        }
    }
}

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn encode_text<O: Output>(&self) -> O::Storage {
        let mut writer = Writer::<O>::new();
        writer.value(self);
        writer.into_inner()
    }
}
