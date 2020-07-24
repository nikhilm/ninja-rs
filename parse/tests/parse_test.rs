use insta::{assert_debug_snapshot, assert_display_snapshot};
use ninja_parse::{build_representation, Loader};
use std::{ffi::OsStr, fs, os::unix::ffi::OsStrExt, path::Path};

/* This bit is a copy of the glob_exec function in insta until insta#119 is fixed*/

use globwalk::{FileType, GlobWalkerBuilder};

use insta::Settings;

pub struct SimpleFileLoader {}

impl Loader for SimpleFileLoader {
    fn load(&mut self, from: Option<&[u8]>, request: &[u8]) -> std::io::Result<Vec<u8>> {
        let path = if let Some(from) = from {
            let src_path = Path::new(OsStr::from_bytes(from));
            let req_path = Path::new(OsStr::from_bytes(request));
            if req_path.components().count() > 1 {
                todo!("handle relative paths");
            } else {
                src_path.with_file_name(req_path)
            }
        } else {
            Path::new(OsStr::from_bytes(request)).to_owned()
        };
        std::fs::read(path)
    }
}
pub fn glob_exec<F: FnMut(&Path)>(base: &Path, pattern: &str, mut f: F) {
    let walker = GlobWalkerBuilder::new(base, pattern)
        .case_insensitive(true)
        .file_type(FileType::FILE)
        .build()
        .unwrap();

    for file in walker {
        let file = file.unwrap();
        let path = file.path();

        let mut settings = Settings::clone_current();
        settings.set_input_file(&path);
        settings.set_snapshot_suffix(path.file_name().unwrap().to_str().unwrap());

        settings.bind(|| {
            f(path);
        });
    }
}
/* end */

#[test]
fn test_inputs() {
    // MANIFEST_DIR points to crate, but file! is workspace relative.
    // Pop a component so it all works out.
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(file!())
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap();

    std::env::set_current_dir(&base).unwrap();

    glob_exec(&base, "parse_inputs/*.ninja", |path| {
        // Make input paths relative so running tests on different machines won't mess them up.
        let path = path.strip_prefix(&base).unwrap();
        eprintln!("File {:?}", path);
        let mut loader = SimpleFileLoader {};

        let res = build_representation(&mut loader, path.as_os_str().as_bytes().to_vec());
        match res {
            Ok(ast) => assert_debug_snapshot!(ast),
            Err(e) => assert_display_snapshot!(e),
        };
    });
}
