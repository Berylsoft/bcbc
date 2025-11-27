// TODO impl a more general one outside

use crate::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// currently no need for Default & new()
// TODO impl<B: AsRef<[u8]>, S: AsRef<str>> PartialEq&PartialOrd<S> for ByteStr<B>
// cannot due to similar problem with Value
pub struct ByteStr<B> {
    // Invariant: bytes contains valid UTF-8
    bytes: B,
}

// TODO(std) impl<'a> From<&'a str> for &'a [u8]
/*
impl<'a, B: From<&'a str> + ByteStorage> From<&'a str> for ByteStr<B> {
    fn from(value: &'a str) -> Self {
        // Invariant: value is a str so contains valid UTF-8.
        ByteStr { bytes: value.into() }
    }
}
*/

// remove when generic impl feasible
impl<'a> From<&'a str> for ByteStr<&'a [u8]> {
    fn from(value: &'a str) -> Self {
        // Invariant: value is a str so contains valid UTF-8.
        ByteStr { bytes: value.as_bytes() }
    }
}

#[cfg(feature = "alloc")]
impl From<String> for ByteStr<Vec<u8>> {
    fn from(value: String) -> Self {
        // Invariant: value is a String so contains valid UTF-8.
        ByteStr { bytes: value.into_bytes() }
    }
}

// remove when generic impl feasible
#[cfg(feature = "bytes")]
impl ByteStr<Bytes> {
    // impl From<&'static str> for ByteStr<Bytes> if not conflict with below
    pub const fn from_static(value: &'static str) -> Self {
        ByteStr {
            // Invariant: value is a str so contains valid UTF-8.
            // bytes: Bytes::from(value), // no const trait fn
            bytes: Bytes::from_static(value.as_bytes()),
        }
    }
}

// remove when generic impl feasible
#[cfg(feature = "bytes")]
impl<'a> From<&'a str> for ByteStr<Bytes> {
    fn from(value: &'a str) -> Self {
        ByteStr {
            // Invariant: value is a str so contains valid UTF-8.
            // bytes: Bytes::from(value), // not impled? what about below tests?
            bytes: Bytes::copy_from_slice(value.as_bytes()),
        }
    }
}

#[cfg(all(feature = "alloc", feature = "bytes"))]
impl From<String> for ByteStr<Bytes> {
    fn from(value: String) -> Self {
        // Invariant: value is a String so contains valid UTF-8.
        ByteStr { bytes: value.into() }
    }
}

impl<B: AsRef<[u8]>> ByteStr<B> {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl<B: AsRef<[u8]> + ByteStorage> ByteStr<B> {
    pub fn as_str(&self) -> &str {
        let b: &[u8] = self.bytes.as_ref();
        // Safety: the invariant of `bytes` is that it contains valid UTF-8.
        unsafe { core::str::from_utf8_unchecked(b) }
    }

    // DO NOT impl TryFrom<B> keeping consistency to std
    // https://internals.rust-lang.org/t/20078/
    // TODO Result<Self, (B, core::str::Utf8Error)> ?
    pub fn from_utf8(bytes: B) -> Result<Self, core::str::Utf8Error> {
        let _ = core::str::from_utf8(bytes.as_ref())?;
        Ok(ByteStr { bytes })
    }

    // cannot impl<B> From<ByteStr<B>> for B
    pub fn leak_bytes(self) -> B {
        self.bytes
    }
}

impl<B: AsRef<[u8]>> AsRef<[u8]> for ByteStr<B> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<B: AsRef<[u8]> + ByteStorage> AsRef<str> for ByteStr<B> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<B> ByteStr<B> {
    pub fn map_bytes<B2>(self, f: fn(B) -> B2) -> ByteStr<B2> {
        ByteStr { bytes: f(self.bytes) }
    }
}

impl<B: AsRef<[u8]> + ByteStorage> core::fmt::Debug for ByteStr<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<B: AsRef<[u8]> + ByteStorage> core::fmt::Display for ByteStr<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}

#[test]
#[allow(path_statements)]
const fn test() {
    trait AssertImpl { const ASSERT: () = (); }

    struct A<T>(T);
    impl<'a, T: From<&'a str>> AssertImpl for A<T> { const ASSERT: () = (); }
    // A::<&[u8]>::ASSERT;
    A::<Vec<u8>>::ASSERT;
    // A::<Box<[u8]>>::ASSERT;
    A::<Bytes>::ASSERT;
    A::<ByteStr<&[u8]>>::ASSERT;
    A::<ByteStr<Bytes>>::ASSERT;

    struct B<T>(T);
    impl<T: From<&'static str>> AssertImpl for B<T> { const ASSERT: () = (); }
    // B::<&'static [u8]>::ASSERT;
    B::<Bytes>::ASSERT;
    B::<ByteStr<&'static [u8]>>::ASSERT;
    B::<ByteStr<Bytes>>::ASSERT;
}
