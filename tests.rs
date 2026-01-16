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
fn s<S: ?Sized + AsRef<str>>(bytes: &S) -> Box<[char]> {
    bytes.as_ref().chars().collect()
}

macro_rules! seq {
    ($($x:expr),* $(,)?) => {
        Box::new([$($x),*])
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
        println!("{}", hex::encode(exp));
        println!("len={}", buf.len());
        println!("{}", hex::encode(&buf));
        assert_eq!(&buf, exp);
        // let v2 = Value::decode::<SliceInput>(&buf).unwrap();
        // assert_eq!(v, v2);
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
            55 00
            50 02
                50 02
                    55 7b
                    4c 02
                        55 00
                        50 02
                            53 05 68656c6c6f
                            53 07 676f6f64627965
                50 02
                    55 bf843d
                    4c 02
                        55 00
                        50 03
                            53 0b 686f772061726520796f75
                            53 04 66696e65
                            53 06 7468616e6b73
        "),
    );
}
