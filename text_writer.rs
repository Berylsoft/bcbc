use crate::{*, byte_storage::Output, leb128::*};

pub(crate) struct Writer<O> {
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

    // fn i_tuple(&'a mut self) -> TupleWriter<'a> {

    // }

    fn value<B: AsRef<[u8]> + ByteStorage>(&mut self, value: &Value<B>) {

    }
}

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn encode_text<O: Output>(&self) -> O::Storage {
        let mut writer = Writer::<O>::new();
        writer.value(self);
        writer.into_inner()
    }
}
