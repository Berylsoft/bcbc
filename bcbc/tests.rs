use hex_literal::hex;
use crate::*;

macro_rules! expb {
    ($s:literal) => {
        hex!($s).as_slice()
    };
}

#[inline(always)]
fn b<B: ?Sized + AsRef<[u8]>>(bytes: &B) -> &[u8] {
    bytes.as_ref()
}

#[inline(always)]
fn s<S: ?Sized + AsRef<str>>(bytes: &S) -> ByteStr<&[u8]> {
    bytes.as_ref().into()
}

macro_rules! seq {
    ($($x:expr),+ $(,)?) => {
        Box::new([$($x),+])
    };
}

macro_rules! println {
    ($($tt:tt)*) => {};
}

#[test]
fn cases() {
    fn case(v: Value<&'static [u8]>, exp: &'static [u8]) {
        println!("{:?}", &v);
        let buf = v.encode::<VecOutput>();
        println!("len={}", exp.len());
        println!("{}", hex::encode(&exp));
        println!("len={}", buf.len());
        println!("{}", hex::encode(&buf));
        assert_eq!(&buf, &exp);
        let v2 = Value::decode::<SliceInput>(&buf).unwrap();
        assert_eq!(v, v2);
        #[cfg(feature = "bytes")]
        {
            // TODO after byte-storage separated this is actually testing byte-storage. should we move this?
            // TODO is `Bytes -> &[u8]` possible?
            // fn bytes2slice<'a>(v: Value<Bytes>) -> Value<&'a [u8]> {
            //     // typeof AsRef::as_ref : for<'a> fn(&'a Bytes) -> &'a [u8]
            //     // possible but limited by map_bytes's signature
            //     v.map_bytes(AsRef::as_ref)
            // }
            // reverse thinking: &'static [u8] -> Bytes (or copying &'a [u8] -> Bytes if necessary in some real world cases).
            // success naturally and similarly efficient.
            let buf = Bytes::from(buf);
            let v3 = Value::decode::<BytesInput>(buf).unwrap();
            assert_eq!(v.map_bytes(Bytes::from_static), v3);
        }
    }

    case(
        Value::Map((Type::U64, Type::List(Box::new(Type::String))), seq![
            (Value::U64(123), Value::List(Type::String, seq![
                Value::String(s("hello")),
                Value::String(s("goodbye")),
            ])),
            (Value::U64(999999), Value::List(Type::String, seq![
                Value::String(s("thanks")),
                Value::String(s("how are you")),
            ])),
        ]),
        expb!("
        b2 06 110e
        03 7b     a2 0e 85 68656c6c6f   87 676f6f64627965
        23 0f423f a2 0e 86 7468616e6b73 8b 686f772061726520796f75
        "),
    );

    case(
        Value::Tuple(seq![
            Value::Unit,
            Value::Bool(false),
            Value::I64(-7777777),
            Value::U64(24393),
            Value::F64(50.0_f64.to_bits()),
            Value::String(s("Berylsoft")),
            Value::Bytes(b(b"(\x00)")),
            Value::Option(Type::String, Box::new(None)),
            Value::Option(Type::Bool, Box::new(Some(Value::Bool(true)))),
            Value::Alias(TypeId::Hash(HashId { hash: hex!("fedcba98765432") }), Box::new(Value::Bytes(b(b"\xff")))),
            Value::CEnum(TypeId::Std(StdId { schema: 0x01, id: 0x5f50 }), 11),
            Value::Enum(TypeId::Std(StdId { schema: 0x01, id: 0x5f49 }), 5, Box::new(Value::I64(5))),
            Value::Enum(TypeId::Std(StdId { schema: 0xfe, id: 0x00aa }), 163, Box::new(Value::U64(12))),
            Value::Type(Type::List(Box::new(Type::List(Box::new(Type::Struct(TypeId::Anonymous)))))),
            Value::TypeId(TypeId::Hash(HashId { hash: hex!("fedcba98765432") })),
            Value::Option(Type::Tuple(seq![Type::I64, Type::Unit, Type::Unknown]), Box::new(Some(Value::Tuple(seq![Value::I64(9), Value::Unit, Value::Bool(true)])))),
        ]),
        expb!("
        cc 10
        0e
        1e
        2a 76adf1
        13 5f49
        1d 4049
        89 426572796c736f6674
        93 280029
        3e 0e
        4e 02 2e
        5e ff fedcba98765432 91 ff
        db 01 5f50
        e5 01 5f49 07 05
        ec a3 fe 00aa 03 0c
        6e 11 11 17 00
        7e ff fedcba98765432
        4e  13 03 0a 01 00  c3 07 09 0e 2e
        "),
    );

    fn err_case(exp: &'static [u8], err: Error, pos: usize) {
        let err2 = Value::decode::<SliceInput>(exp).unwrap_err();
        assert_eq!(err2, FullError { err, buf: exp, pos });
    }

    err_case(
        expb!("7a ffffffffffffffff"),
        Error::BytevarIntSign { buf: [0xff; 8] },
        9,
    );

    err_case(
        expb!("0e 000000"),
        Error::Read(ReadError::TooLong { rest: 3 }),
        1,
    );

    err_case(
        expb!("89 426572796c736f66"),
        Error::Read(ReadError::TooShort { rest: 8, expected: 9 }),
        1,
    );

    err_case(
        expb!("6e ff"),
        Error::Tag(0xff),
        2,
    );

    err_case(
        expb!("82 ffff"),
        Error::Utf8(core::str::from_utf8(expb!("ffff")).unwrap_err()),
        3,
    );

    err_case(
        expb!("8c 00"),
        Error::ExtvarTooLong { l4: EXT8, exp_l4: 0u8.try_into().unwrap(), u: 0 },
        2,
    );

    err_case(
        expb!("21 000001"),
        Error::BytevarLongerThanType { len: 3, nlen: 2, buf: hex!("00 00 00 00 00 00 00 01") },
        4,
    );

    err_case(
        expb!("11 0001"),
        Error::BytevarLongerThanExpected { len: 2, nlen: 2, exp_len: 1, buf: hex!("00 00 00 00 00 00 00 01") },
        3,
    );

    err_case(
        expb!("0a 00"),
        Error::BytevarNegZero { buf: [0; 8] },
        2,
    );
}
