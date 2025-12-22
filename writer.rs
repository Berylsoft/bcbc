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

    fn uleb128(&mut self, mut value: u128) {
        loop {
            let byte = value as u8 & 0x7F;
            value >>= 7;

            if value == 0 {
                self.byte(byte);
                break;
            } else {
                self.byte(byte | 0x80);
            }
        }
    }

    fn sleb128(&mut self, mut value: i128) {
        loop {
            let byte = value as u8 & 0x7F;
            value >>= 7;

            let sign = byte & 0x40;
            if (value == 0 && sign == 0) || (value == -1 && sign != 0) {
                self.byte(byte);
                break;
            } else {
                self.byte(byte | 0x80);
            }
        }
    }

    fn val<B: AsRef<[u8]> + ByteStorage>(&mut self, val: &Value<B>) {
        unimplemented!()
    }
}

impl<B: AsRef<[u8]> + ByteStorage> Value<B> {
    pub fn encode<O: Output>(&self) -> O::Storage {
        let mut writer = Writer::<O>::new();
        writer.val(self);
        writer.into_inner()
    }
}
