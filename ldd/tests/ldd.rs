extern crate ldd;
#[macro_use] extern crate matches;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ldd::Ldd;

#[test]
fn test_apt() {
    let ldd = Ldd::new(".vagga/test").unwrap();
    let deps = ldd.deps("/usr/bin/apt").unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    let expected_deps = vec!(
        Path::new("/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libbz2.so.1.0").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libc.so.6").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libdl.so.2").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libgcc_s.so.1").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/liblzma.so.5").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libm.so.6").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libresolv.so.2").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libz.so.1").to_path_buf(),
        Path::new("/usr/lib/x86_64-linux-gnu/libapt-pkg.so.5.0").to_path_buf(),
        Path::new("/usr/lib/x86_64-linux-gnu/libapt-private.so.0.0").to_path_buf(),
        Path::new("/usr/lib/x86_64-linux-gnu/liblz4.so.1").to_path_buf(),
        Path::new("/usr/lib/x86_64-linux-gnu/libstdc++.so.6").to_path_buf(),
    );
    assert_eq!(deps, expected_deps);
}

#[test]
fn test_java() {
    let ldd = Ldd::new(".vagga/test").unwrap();
    let deps = ldd.deps("/usr/bin/java").unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    let expected_deps = vec!(
        Path::new("/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libc.so.6").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libdl.so.2").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libpthread.so.0").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libz.so.1").to_path_buf(),
        Path::new("/usr/lib/jvm/java-8-openjdk-amd64/jre/lib/amd64/jli/libjli.so").to_path_buf(),
    );
    assert_eq!(deps, expected_deps);
}

#[test]
fn test_python() {
    let ldd = Ldd::new(".vagga/test").unwrap();
    let deps = ldd.deps("/usr/bin/python3").unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    let expected_deps = vec!(
        Path::new("/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libc.so.6").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libdl.so.2").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libexpat.so.1").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libm.so.6").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libpthread.so.0").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libutil.so.1").to_path_buf(),
        Path::new("/lib/x86_64-linux-gnu/libz.so.1").to_path_buf(),
    );
    assert_eq!(deps, expected_deps);
}
