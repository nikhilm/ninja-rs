use insta::{assert_debug_snapshot, assert_display_snapshot};
use ninja_parse::Parser;
use std::fs;
/* This bit is a copy of the glob_exec function in insta until insta#119 is fixed*/
use std::path::Path;

use globwalk::{FileType, GlobWalkerBuilder};

use insta::Settings;

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

    glob_exec(&base, "inputs/*.ninja", |path| {
        eprintln!("File {:?}", path);
        let input = fs::read(path).unwrap();
        let parser = Parser::new(&input, Some(path.as_os_str().to_str().unwrap().to_string()));
        let res = parser.parse();
        match res {
            Ok(ast) => assert_debug_snapshot!(ast),
            Err(e) => assert_display_snapshot!(e),
        };
    });
}
