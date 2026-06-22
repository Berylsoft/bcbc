use hex_literal::hex;
use crate::{*, byte_storage::{SliceInput, VecOutput}};

macro_rules! expb {
    ($s:literal) => {
        hex!($s).as_slice()
    };
}

#[inline]
fn b<B: ?Sized + AsRef<[u8]>>(bytes: &B) -> &[u8] {
    bytes.as_ref()
}

#[inline]
fn s<S: ?Sized + AsRef<str>>(bytes: &S) -> Box<[char]> {
    bytes.as_ref().chars().collect()
}

macro_rules! seq {
    ($($x:expr),* $(,)?) => {
        Box::new([$($x),*])
    };
}

#[test]
fn cases() {
    fn case(v: Value<&'static [u8]>, exp: &'static [u8]) {
        println!("{:?}", &v);
        let buf = v.encode::<VecOutput>();
        println!("len={}", exp.len());
        println!("{}", hex::encode(exp));
        println!("len={}", buf.len());
        println!("{}", hex::encode(&buf));
        assert_eq!(&buf, exp);
        let v2 = Value::decode::<SliceInput>(&buf).unwrap();
        assert_eq!(v, v2);
    }

    case(
        Value::List(
            Type::Tuple(seq![
                Type::Uint,
                Type::List(Box::new(Type::String)),
            ]),
            seq![
                Value::Tuple(seq![
                    Value::Uint(123),
                    Value::List(Type::String, seq![
                        Value::String(s("hello")),
                        Value::String(s("goodbye")),
                    ]),
                ]),
                Value::Tuple(seq![
                    Value::Uint(999999),
                    Value::List(Type::String, seq![
                        Value::String(s("how are you")),
                        Value::String(s("fine")),
                        Value::String(s("thanks")),
                    ]),
                ]),
            ],
        ),
        expb!("
        4c 02
            46 01
            4d 02
                50 02
                    55 7b
                    4c 02
                        46 01
                        4d 02
                            53 05 68656c6c6f
                            53 07 676f6f64627965
                50 02
                    55 bf843d
                    4c 02
                        46 01
                        4d 03
                            53 0b 686f772061726520796f75
                            53 04 66696e65
                            53 06 7468616e6b73
        "),
    );

    const F64_BYTES: &[u8] = 50.0_f64.to_le_bytes().as_slice();

    case(
        Value::Tuple(seq![
            Value::Tuple(seq![]),
            Value::Bool(false),
            Value::Int(-7777777),
            Value::Uint(1027),
            Value::Uints(seq![11, 12, 1314, 1516171819, 20]),
            Value::Alias(TypeId::Std(10), seq![], Box::new(Value::Bytes(F64_BYTES))),
            Value::String(s("Berylsoft")),
            Value::Bytes(b(b"(\x00)")),
            Value::Option(Type::String, None),
            Value::Option(Type::Bool, Some(Box::new(Value::Bool(true)))),
            Value::Alias(TypeId::Anonymous/* third-party */, seq![], Box::new(Value::Bytes(b(b"\xff")))),
            Value::Enum(TypeId::Std(0x5f50), 11),
            Value::Choice(TypeId::Std(0x5f49), seq![], 5, Box::new(Value::Int(5))),
            Value::Choice(TypeId::Std(0xfe00aa), seq![Type::Alias(TypeId::Std(0xfe00bb), seq![Type::Uint])], 163, Box::new(Value::Uint(12))),
            Value::Type(Type::List(Box::new(Type::List(Box::new(Type::Struct(TypeId::Anonymous, seq![])))))),
            Value::TypeId(TypeId::Std(0xfedcba98765432/* third-party */)),
            Value::Option(
                Type::Tuple(seq![Type::Int, Type::Tuple(seq![Type::Bytes]), Type::Bool]),
                Some(Box::new(Value::Tuple(seq![Value::Int(9), Value::Tuple(seq![Value::Bytes(b(b"\xab"))]), Value::Bool(true)])))
            ),
        ]),
        expb!("
        50 11
            50 00
            46 00
            49 8fa4a57c
            55 8308
            4e 05 0b 0c a20a abe4fbd205 14
            41 03
                44 02
                    55 79
                    55 0a
                47 00
                42 08 0000000000004940
            53 09 426572796c736f6674
            42 03 280029
            4f 02
                46 00
                54 02
                    55 73
                    50 00
            4f 02
                46 01
                46 01
            41 03
                44 02
                    55 78
                    50 00
                47 00
                42 01 ff
            45 02
                44 02
                    55 79
                    55 d0be01
                55 0b
            43 04
                44 02
                    55 79
                    55 c9be01
                47 00
                55 05
                49 05
            43 04
                44 02
                    55 79
                    55 aa81f807
                47 01
                    54 02
                        55 61
                        50 02
                            44 02
                                55 79
                                55 bb81f807
                            47 01
                                54 02
                                    55 75
                                    50 00
                55 a301
                55 0c
            54 02
                55 6c
                54 02
                    55 6c
                    54 02
                        55 72
                        50 02
                            44 02
                                55 78
                                50 00
                            47 00
            44 02
                55 79
                55 b2a8d9c3a997b77f
            4f 02
                46 01
                50 03
                    49 09
                    50 01
                        42 01 ab
                    46 01
        ")
    );

    fn err_case(exp: &'static [u8], err: Error, pos: usize) {
        let err2 = Value::decode::<SliceInput>(exp).unwrap_err();
        assert_eq!(err2, FullError { err, buf: exp, pos });
    }

    err_case(
        expb!("ff"),
        Error::Tag(0xff),
        1,
    );

    err_case(
        expb!("54 02  55 31"),
        Error::TypeTag(0x31),
        4,
    );

    err_case(
        expb!("44 02  55 31"),
        Error::TypeIdTag(0x31),
        4,
    );

    err_case(
        expb!("46 02"),
        Error::U8ToBool(0x02),
        2,
    );

    // TODO leb128 tests

    // TODO all max_len tests

    err_case(
        expb!("53 01 ffffffff0f"),
        Error::U32ToChar(0xffffffff),
        7,
    );

    macro_rules! multi_fixed_tuple_len {
        ([$($exp:expr => $exp_len:expr,)*], $len:expr, $pos:expr,) => {
            $(err_case(
                expb!($exp),
                Error::FixedTupleLen { len: $len, exp_len: $exp_len },
                $pos,
            );)*
        };
    }

    multi_fixed_tuple_len! {
        [
            "4c 0f" => 2,
            "4f 0f" => 2,
            "41 0f" => 3,
            "45 0f" => 2,
            "43 0f" => 4,
            "52 0f" => 3,
            "54 0f" => 2,
            "44 0f" => 2,
        ],
        0x0f,
        2,
    };

    err_case(
        expb!("4c 00"),
        Error::FixedTupleLen { len: 0x00, exp_len: 2 },
        2,
    );

    err_case(
        expb!("4c ff01"),
        Error::FixedTupleLen { len: 0xff, exp_len: 2 },
        3,
    );

    // other cases that appear to be leb128 error?

    err_case(
        expb!("4c 8002"),
        Error::ULEB128LongerThanTargetType(0x0100, "u8"),
        3,
    );

    // for value API we can only detect inner type
    // TODO how to test low level API?

    err_case(
        expb!("4c 02  46 01  50"),
        Error::ExpectedTypeMismatch { tag: Tag::Tuple, exp_tag: Tag::ListItems },
        5,
    );

    err_case(
        expb!("
        41 03
            44 02
                55 78
                50 00
            50
        "),
        Error::ExpectedTypeMismatch { tag: Tag::Tuple, exp_tag: Tag::Generics },
        9,
    );

    err_case(
        expb!("4c 02  46 01  4d 00"),
        Error::EmptyListInNotEmptyMark,
        6,
    );

    err_case(
        expb!("4d"),
        Error::ImplicitTypeOnTop(Tag::ListItems),
        1,
    );

    err_case(
        expb!("47"),
        Error::ImplicitTypeOnTop(Tag::Generics),
        1,
    );
}
