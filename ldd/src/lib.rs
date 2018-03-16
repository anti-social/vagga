extern crate elfkit;
extern crate glob;
#[macro_use] extern crate quick_error;

use std::collections::{BTreeSet, HashSet};
use std::io::{self, BufReader, BufRead};
use std::ffi::OsString;
use std::fs::{File, read_link, symlink_metadata};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use elfkit::Elf;
use elfkit::types::{DynamicType, SectionType};
use glob::glob;

quick_error! {
    #[derive(Debug)]
    pub enum LddError {
        Io(err: io::Error) {
            from()
        }
        Elf(err: elfkit::Error) {
            from()
        }
    }
}

#[derive(Clone)]
struct LdContext {
    origin: PathBuf,    
    lpaths: Vec<PathBuf>,
    seen_lpaths: HashSet<PathBuf>,
}

impl LdContext {
    fn new(origin: &Path) -> LdContext {
        LdContext {
            origin: origin.to_path_buf(),
            lpaths: vec!(),
            seen_lpaths: HashSet::new(),
        }
    }

    fn add_lpath(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if !self.seen_lpaths.contains(&path) {
            self.lpaths.push(path.clone());
            self.seen_lpaths.insert(path);
        }
    }

    // fn iter_lpaths(&)    
}

#[derive(Debug)]
pub struct Ldd {
    sysroot: PathBuf,
    lpaths: Vec<PathBuf>,
}

impl Ldd {
    pub fn new<P: AsRef<Path>>(sysroot: P) -> io::Result<Ldd> {
        let sysroot = sysroot.as_ref().to_path_buf();
        let lpaths = parse_ld_so_conf(&sysroot, &sysroot.join("etc/ld.so.conf"))?;
        Ok(Ldd {
            sysroot,
            lpaths,
        })
    }

    pub fn deps<P: AsRef<Path>>(&self, path: P) -> Result<BTreeSet<PathBuf>, LddError> {
        let ref path = join(&self.sysroot, path.as_ref());
        let mut lpaths = self.lpaths.clone();
        println!("Lpaths: {:?}", &lpaths);
//        let mut ctx = LdContext::new(&self.get_origin_dir(path)?);
        let mut ctx = LdContext::new(Path::new(""));
        for lpath in lpaths {
            ctx.add_lpath(&lpath);
        }
        let mut deps = BTreeSet::new();
        self.find_deps(path, &mut ctx, &mut deps)?;
        Ok(deps)
    }

    fn find_deps(&self, path: &Path, ctx: &mut LdContext, deps: &mut BTreeSet<PathBuf>) -> Result<(), LddError> {
        println!("Finding deps for: {:?}", path);    
        let mut src_file = File::open(path)?;
        let mut src_elf = Elf::from_reader(&mut src_file)?;
        let mut neededs = Vec::new();
        for shndx in 0..src_elf.sections.len() {
            if src_elf.sections[shndx].header.shtype == SectionType::DYNAMIC {
                src_elf.load(shndx, &mut src_file)?;
                let dynamic = match src_elf.sections[shndx].content.as_dynamic() {
                    Some(dynamic) => dynamic,
                    None => continue,
                };

                for dyn in dynamic.iter() {
                    if dyn.dhtype == DynamicType::RPATH {
                        if let elfkit::dynamic::DynamicContent::String(ref name) = dyn.content {
                            let rpaths_str = String::from_utf8_lossy(&name.0).into_owned();
//                            for rpath_str in rpaths_str.split(':').reverse() {
//                                ctx.add_lpath(
//                                    &self.sysroot.join(&)
//                                )
//                            }
                        }
                    }
                    if dyn.dhtype == DynamicType::NEEDED {
                        if let elfkit::dynamic::DynamicContent::String(ref name) = dyn.content {
                            // println!("Dep: {}", String::from_utf8_lossy(&name.0));
                            neededs.push(PathBuf::from(String::from_utf8_lossy(&name.0).into_owned()));
                        }
                    }
                }
            }
        }

        for needed in &neededs {
            for lpath in &ctx.lpaths {
                let dep = join(&self.sysroot, &lpath.join(needed));
                // println!("Checking {:?}", &dep);
                if dep.exists() {
                    if (deps.insert(dep.clone())) {
                        println!("Found: {:?}", &dep);
                        self.find_deps(&dep, &mut ctx.clone(), deps);
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

fn join(sysroot: &Path, path: &Path) -> PathBuf {
    sysroot.join(
        match path.strip_prefix("/") {
            Ok(rel_path) => rel_path,
            Err(_) => path,
        }
    )
}

fn follow_link(sysroot: &Path, path: &Path) -> io::Result<PathBuf> {
    let stat = symlink_metadata(path)?;
    if stat.file_type().is_symlink() {
        let ref dst_path = join(sysroot, &read_link(path)?);
        follow_link(sysroot, dst_path)
    } else {
        Ok(path.to_path_buf())
    }
}

fn parse_ld_so_conf(sysroot: &Path, path: &Path) -> io::Result<Vec<PathBuf>> {
    println!("ld.so.conf: {:?}", path);
    let mut paths = Vec::new();

    println!("> following link {:?}", path);
    let ref path = follow_link(sysroot, path)?;
    println!("> opening {:?}", path);
    let f = File::open(path)?;
    println!("> open ok");
    let f = BufReader::new(&f);
    for line in f.lines() {
        let line = line?;
        let line = line.trim();
        if line.starts_with("#") {
            continue;
        }
        if line == "" {
            continue;
        }

        if line.contains(" ") {
            println!("{}", line);
            if line.starts_with("include ") {
                let include_path = join(sysroot, &PathBuf::from(line.split(" ").last().unwrap()));
                for entry in glob(include_path.to_str().unwrap()).expect("Failed to read glob pattern") {
                    let include_path = PathBuf::from(entry.unwrap().to_string_lossy().into_owned());
                    paths.extend(parse_ld_so_conf(sysroot, &include_path)?);
                }
            }
        } else {
            paths.push(PathBuf::from(line.to_owned()));
        }
    }
    Ok(paths)
}