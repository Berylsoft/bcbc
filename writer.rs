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
    fn bytes<B2: AsRef<[u8]>>(&mut self, bytes: B2) {
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

    fn uints(&mut self, uints: Box<[u128]>) {
        for n in uints {
            self.uleb128(n);
        }
    }

    fn string(&mut self, usvs: Box<[u32]>) {
        for n in usvs {
            self.uleb128(n as u128);
        }
    }

    fn tuple<B: AsRef<[u8]> + ByteStorage>(&mut self, values: Box<[Value<B>]>) {
        self.uleb128(values.len() as u128);
        for value in values {
            self.value(value);
        }
    }

    fn ty<B: AsRef<[u8]> + ByteStorage>(&mut self, ty: Type) {
        self.value(ty.encode::<B>());
    }

    fn list<B: AsRef<[u8]> + ByteStorage>(&mut self, ty: Type, values: Box<[Value<B>]>) {
        self.value(Value::Tuple(Box::new([
            Value::Type(ty),
            Value::Tuple(values)
        ])));
    }

    fn value<B: AsRef<[u8]> + ByteStorage>(&mut self, value: Value<B>) {
        self.byte(value.as_tag() as u8);
        match value {
            Value::Uint(n) => self.uleb128(n),
            Value::Int(n) => self.sleb128(n),
            Value::Uints(uints) => self.uints(uints),
            Value::Bytes(bytes) => self.bytes(bytes),
            Value::String(usvs) => self.string(usvs),
            Value::Tuple(values) => self.tuple(values),

            Value::List(ty, values) => self.list(ty, values),
            Value::Alias(type_id, value) => todo!(),
            Value::Enum(type_id, var_id) => todo!(),
            Value::Choice(type_id, var_id, value) => todo!(),
            Value::Struct(type_id, values) => todo!(),
            Value::Type(ty) => self.ty::<B>(ty),
            Value::TypeId(type_id) => todo!(),
        }
    }
}

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn encode<O: Output>(self) -> O::Storage {
        let mut writer = Writer::<O>::new();
        writer.value(self);
        writer.into_inner()
    }
}
