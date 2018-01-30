extern crate ldd;

use ldd::Ldd;

#[test]
fn test_apt() {
    let ldd = Ldd::new("ubuntu-xenial").unwrap();
    let deps = ldd.deps("/usr/bin/apt");
    for dep in deps.unwrap() {
        println!("{:?}", dep);
    }
}
