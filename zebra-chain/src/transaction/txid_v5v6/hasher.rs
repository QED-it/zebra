use std::io;

use byteorder::{LittleEndian, WriteBytesExt};

use blake2b_simd::{Hash as Blake2bHash, Params, State};

use crate::serialization::ZcashSerialize;

pub(crate) struct Hasher(State);

impl Hasher {
    pub fn new(personal: &[u8; 16]) -> Self {
        Self(Params::new().hash_length(32).personal(personal).to_state())
    }

    pub fn add<Digest: HashWriter>(mut self, digest: Digest) -> Result<Self, io::Error> {
        digest.write(&mut self.0)?;
        Ok(self)
    }

    pub fn add_all_if_any_nonempty(mut self, child_hashers: &[Self]) -> Result<Self, io::Error> {
        if child_hashers.iter().any(|child| !child.is_empty()) {
            for child in child_hashers {
                child.write(&mut self.0)?
            }
        }
        Ok(self)
    }

    pub fn is_empty(&self) -> bool {
        self.0.count() == 0
    }

    // FIXME: ref to self is used here instead of self only to make
    // impl HashWriter for Hasher working - is this correct?
    pub fn finalize(&self) -> Blake2bHash {
        self.0.finalize()
    }
}

pub(crate) trait HashWriter {
    fn write<W: io::Write>(&self, writer: W) -> Result<(), io::Error>;
}

impl HashWriter for Hasher {
    fn write<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
        writer.write_all(self.finalize().as_bytes())
    }
}

impl HashWriter for u32 {
    fn write<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
        writer.write_u32::<LittleEndian>(*self)
    }
}

impl HashWriter for &[u8] {
    fn write<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
        writer.write_all(self)
    }
}

impl<const N: usize> HashWriter for [u8; N] {
    fn write<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
        writer.write_all(self)
    }
}

impl<T: ZcashSerialize> HashWriter for T {
    fn write<W: io::Write>(&self, writer: W) -> Result<(), io::Error> {
        self.zcash_serialize(writer)
    }
}
