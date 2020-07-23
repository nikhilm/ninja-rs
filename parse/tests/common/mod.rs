use ninja_parse::{build_representation, Loader};
use std::{ffi::OsStr, fs, os::unix::ffi::OsStrExt};

pub struct DummyLoader {}

impl Loader for DummyLoader {
    fn load(&mut self, _from: Option<&[u8]>, _load: &[u8]) -> std::io::Result<Vec<u8>> {
        unimplemented!();
    }
}

pub struct SimpleFileLoader {}

impl Loader for SimpleFileLoader {
    fn load(&mut self, from: Option<&[u8]>, load: &[u8]) -> std::io::Result<Vec<u8>> {
        assert!(from.is_none());
        fs::read(OsStr::from_bytes(load))
    }
}
