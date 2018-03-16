extern crate ldd;

use ldd::Ldd;

#[test]
fn test_apt() {
    let ldd = match Ldd::new(".vagga/test") {
        Ok(ldd) => ldd,
        Err(e) => panic!("{}", e),
    };
    let deps = ldd.deps("/usr/bin/apt");
    for dep in deps.unwrap() {
        println!("{:?}", dep);
    }
}

//#[test]
//fn test_java() {
//    let ldd = Ldd::new(".vagga/test").unwrap();
//    for dep in ldd.deps("/usr/bin/java").unwrap() {
//        println!("{:?}", dep);
//    }
//}
