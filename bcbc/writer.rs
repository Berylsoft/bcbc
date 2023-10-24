use foundations::byterepr::*;
use super::*;

// TODO writer error?

#[inline]
pub fn usize_u64(n: usize) -> u64 {
    n.try_into().expect("FATAL: usize length to u64 error")
}

macro_rules! num_impl {
    ($($num:tt)*) => {$(
        fn $num(&mut self, n: $num) {
            self.bytes(n.to_bytes());
        }
    )*};
}

fn byteuvar_len(buf: &[u8; 8]) -> usize {
    for (i, b) in buf.iter().enumerate() {
        if *b != 0 {
            return 8 - i;
        }
    }
    1
}

fn bytefvar_len(buf: &[u8; 8]) -> usize {
    for (i, b) in buf.iter().rev().enumerate() {
        if *b != 0 {
            return 8 - i;
        }
    }
    1
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Writer {
        Writer { bytes: Vec::new() }
    }

    fn into_bytes(self) -> Vec<u8> {
        let Writer { bytes } = self;
        bytes
    }

    #[inline]
    fn bytes<B: AsRef<[u8]>>(&mut self, bytes: B) {
        self.bytes.extend_from_slice(bytes.as_ref());
    }

    #[inline]
    fn u8(&mut self, n: u8) {
        self.bytes.push(n);
    }

    num_impl! {
        u16 u32 u64
    }

    fn typeid(&mut self, id: &TypeId) {
        // TODO(styling): as_h8 or in match blocks?
        self.u8(id.as_h8());
        match id {
            TypeId::Std(std_id) => {
                self.u16(std_id.id());
            },
            TypeId::Hash(hash_id) => {
                self.bytes(hash_id.hash());
            },
            TypeId::Anonymous => {},
        }
    }

    fn ty(&mut self, t: &Type) {
        self.u8(t.as_tag() as u8);
        match t {
            Type::Unknown |
            Type::Unit |
            Type::Bool |
            Type::U8 |
            Type::U16 |
            Type::U32 |
            Type::U64 |
            Type::I8 |
            Type::I16 |
            Type::I32 |
            Type::I64 |
            Type::F16 |
            Type::F32 |
            Type::F64 |
            Type::String |
            Type::Bytes |
            Type::Type |
            Type::TypeId => {},

            Type::Option(t) |
            Type::List(t) => {
                self.ty(t);

            },
            Type::Map(tk, tv) => {
                self.ty(tk);
                self.ty(tv);

            },

            Type::Tuple(s) => {
                // should checked in type check
                self.u8(s.len().try_into().unwrap());

                for t in s {
                    self.ty(t);
                }

            }

            Type::Alias(r) |
            Type::CEnum(r) |
            Type::Enum(r) |
            Type::Struct(r) => {
                self.typeid(r);

            },
        }
    }

    #[inline]
    fn header(&mut self, h4: H4, l4: L4) {
        self.u8(casting::from_h4l4(h4, l4));
    }

    fn ext1(&mut self, ext1: Ext1) {
        self.header(H4::from_ext1(ext1), L4::EXT1);
    }

    fn extvar(&mut self, h4: H4, u: u64) {
        // TODO casting using overflow protected methods?
        if u < (EXT8 as u64) {
            self.header(h4, (u as u8).try_into().unwrap());
        } else if u <= (u8::MAX as u64) {
            self.header(h4, EXT8);
            self.u8(u as u8);
        } else if u <= (u16::MAX as u64) {
            self.header(h4, EXT16);
            self.u16(u as u16);
        } else if u <= (u32::MAX as u64) {
            self.header(h4, EXT32);
            self.u32(u as u32);
        } else {
            self.header(h4, EXT64);
            self.u64(u);
        }
    }

    fn extszvar(&mut self, h4: H4, sz: usize) {
        self.extvar(h4, usize_u64(sz))
    }

    fn val_seq(&mut self, s: &Vec<Value>) {
        for v in s {
            self.val(v);
        }
    }

    fn val_seq_map(&mut self, s: &Vec<(Value, Value)>) {
        for (k, v) in s {
            self.val(k);
            self.val(v);
        }
    }

    fn val(&mut self, val: &Value) {
        macro_rules! numval_impl {
            // TODO(Rust): macro on match arms
            (
                U {$($uname:ident $uty:tt)*}
                I8 {$($i8name:ident $i8uty:tt)*}
                I {$($iname:ident $pname:ident $nname:ident $iuty:tt)*}
                F {$($fname:ident $fty:tt)*}
                $($tt:tt)*
            ) => {
                match val {
                    $(Value::$uname(u) => {
                        let mut buf = [0; 8];
                        const NLEN: usize = core::mem::size_of::<$uty>();
                        buf[(8 - NLEN)..].copy_from_slice(&u.to_bytes());
                        let len = byteuvar_len(&buf);
                        self.header(H4::from_bytevar_len(len).unwrap(), L4::$uname);
                        self.bytes(&buf[(8 - len)..]);
                    })*,
                    $(Value::$i8name(i) => {
                        let mut buf = [0; 8];
                        const NLEN: usize = core::mem::size_of::<$i8uty>();
                        buf[(8 - NLEN)..].copy_from_slice(&i.to_bytes());
                        let len = byteuvar_len(&buf);
                        self.header(H4::from_bytevar_len(len).unwrap(), L4::$i8name);
                        self.bytes(&buf[(8 - len)..]);
                    })*,
                    $(Value::$iname(i) => {
                        let l4 = if i.is_positive() {
                            L4::$pname
                        } else {
                            L4::$nname
                        };
                        let u = i.unsigned_abs();
                        let mut buf = [0; 8];
                        const NLEN: usize = core::mem::size_of::<$iuty>();
                        buf[(8 - NLEN)..].copy_from_slice(&u.to_bytes());
                        let len = byteuvar_len(&buf);
                        self.header(H4::from_bytevar_len(len).unwrap(), l4);
                        self.bytes(&buf[(8 - len)..]);
                    })*,
                    $(Value::$fname(f) => {
                        let mut buf = [0; 8];
                        const NLEN: usize = core::mem::size_of::<$fty>();
                        buf[..NLEN].copy_from_slice(&f.to_bytes());
                        let len = bytefvar_len(&buf);
                        self.header(H4::from_bytevar_len(len).unwrap(), L4::$fname);
                        self.bytes(&buf[..len]);
                    })*,
                    $($tt)*
                }
            };
        }

        numval_impl! {
            U {
                U8 u8
                U16 u16
                U32 u32
                U64 u64
            }
            I8 {
                I8 u8
            }
            I {
                I16 P16 N16 u16
                I32 P32 N32 u32
                I64 P64 N64 u64
            }
            F {
                F16 u16
                F32 u32
                F64 u64
            }
            Value::Unit => {
                self.ext1(Ext1::Unit);
            },
            Value::Bool(b) => {
                if *b {
                    self.ext1(Ext1::True)
                } else {
                    self.ext1(Ext1::False);
                }
            },
            Value::String(b) => {
                self.extszvar(H4::String, b.len());
                self.bytes(b);
            },
            Value::Bytes(b) => {
                self.extszvar(H4::Bytes, b.len());
                self.bytes(b);
            },
            Value::Option(t, opt) => {
                if let Some(v) = opt.as_ref() {
                    self.ext1(Ext1::Some);
                    self.ty(t);
                    self.val(v);
                } else {
                    self.ext1(Ext1::None);
                    self.ty(t);
                }
            },
            Value::List(t, s) => {
                self.extszvar(H4::List, s.len());
                self.ty(t);
                self.val_seq(s);
            },
            Value::Map((tk, tv), s) => {
                self.extszvar(H4::Map, s.len());
                self.ty(tk);
                self.ty(tv);
                self.val_seq_map(s);
            },
            Value::Tuple(s) => {
                self.extszvar(H4::Tuple, s.len());
                self.val_seq(s);
            },
            Value::Alias(r, v) => {
                self.ext1(Ext1::Alias);
                self.typeid(r);
                self.val(v);
            },
            Value::CEnum(r, ev) => {
                self.extvar(H4::CEnum, *ev as u64);
                self.typeid(r);
            },
            Value::Enum(r, ev, v) => {
                self.extvar(H4::Enum, *ev as u64);
                self.typeid(r);
                self.val(v);
            },
            Value::Struct(r, s) => {
                self.extszvar(H4::Struct, s.len());
                self.typeid(r);
                self.val_seq(s);
            },
            Value::Type(t) => {
                self.ext1(Ext1::Type);
                self.ty(t);
            },
            Value::TypeId(r) => {
                self.ext1(Ext1::TypeId);
                self.typeid(r);
            },
        }
    }
}

impl Value {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.val(self);
        writer.into_bytes()
    }
}
