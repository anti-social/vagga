extern crate elfkit;
extern crate glob;
#[macro_use] extern crate quick_error;

use std::collections::{BTreeSet, HashSet};
use std::io::{self, BufReader, BufRead};
use std::ffi::OsString;
use std::fs::{File, read_link, symlink_metadata};
use std::path::{Path, PathBuf, Component, MAIN_SEPARATOR};
use std::rc::Rc;

use elfkit::Elf;
use elfkit::types::{DynamicType, SectionType};
use elfkit::dynamic::DynamicContent;

use glob::glob;

use quick_error::ResultExt;

const DEFAULT_LIBRARY_PATHS: &[&Path] = &[
    Path::new("/usr/local/lib"),
    Path::new("/usr/lib"),
    Path::new("/lib"),
];

quick_error! {
    #[derive(Debug)]
    pub enum LddError {
        Io(filename: PathBuf, err: io::Error) {
            display("{:?}: {}", filename, err)
            context(path: &'a Path, err: io::Error)
                -> (path.to_path_buf(), err)
        }
        Elf(err: elfkit::Error) {
            from()
        }
    }
}

#[derive(Clone)]
struct LPaths {
    // Directories listed in the executable's rpath
    rpaths: Vec<PathBuf>,
    // Directories from the LD_LIBRARY_PATH environment variable
    libpaths: Vec<PathBuf>,
    // Directories listed in the executable's rpath
    runpaths: Vec<PathBuf>,
    // Directories from /etc/ld.so.conf
    lpaths: Vec<PathBuf>,
    // Default system libraries
    defaultpaths: Vec<PathBuf>,
}

impl LPaths {
    fn new() -> LPaphs {
        LPaths {
            rpaths: vec!(),
            libpaths: vec!(),
            runpaths: vec!(),
            lpaths: vec!(),
            defaultpaths: DEFAULT_LIBRARY_PATHS.iter()
                .map(|p| p.to_path_buf())
                .collect(),
        }
    }

    fn add_rpath(&mut self, path: PathBuf) {
        if !self.seen_rpaths.contains(&path) {
            self.lpaths.push(path.clone()3421);
            self.seen_lpaths.insert(path);
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
    lpaths: LPaths,
}

impl Ldd {
    pub fn new<P: AsRef<Path>>(sysroot: P) -> Result<Ldd, LddError> {
        let sysroot = sysroot.as_ref().to_path_buf();
        let ld_so_conf_path = Path::new("/etc/ld.so.conf");
        let ref real_ld_so_conf_path = resolve_link(&sysroot, ld_so_conf_path)?;
        let lpaths = LPaths::new();
        if real_ld_so_conf_path.exists() {
            let lpaths = parse_ld_so_conf(
                &sysroot, ld_so_conf_path
            )?;
        }
        Ok(Ldd {
            sysroot,
            lpaths,
        })
    }

    pub fn deps<P: AsRef<Path>>(&self, rel_path: P)
        -> Result<BTreeSet<PathBuf>, LddError>
    {
        let mut lpaths = self.lpaths.clone();
        println!("Lpaths: {:?}", &lpaths);
//        let mut ctx = LdContext::new(&self.get_origin_dir(path)?);
        let mut ctx = LdContext::new(Path::new(""));
        for lpath in lpaths {
            ctx.add_lpath(&lpath);
        }
        let mut deps = BTreeSet::new();
        self.find_deps(rel_path.as_ref(), &mut ctx, &mut deps)?;
        Ok(deps)
    }

    fn find_deps(&self,
                 rel_path: &Path, ctx: &mut LdContext, deps: &mut BTreeSet<PathBuf>)
        -> Result<(), LddError>
    {
        println!("Finding deps for: {:?}", rel_path);
        let ref path = resolve_link(&self.sysroot, rel_path)?;
        println!("Real file path: {:?}", path);
        let ref origin_dir = Path::new("/").join(path.parent().unwrap().strip_prefix(&self.sysroot).unwrap());
        println!("Origin: {:?}", origin_dir);
        let mut src_file = File::open(path).context(path.as_path())?;
        println!("Opened!");
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
                    match dyn.dhtype {
                        DynamicType::RUNPATH => {}
                        DynamicType::RPATH => {
                            if let DynamicContent::String(ref name) = dyn.content {
                                let rpaths_str = String::from_utf8_lossy(&name.0).into_owned();
                                println!("RPATH: {}", rpaths_str);
                                for rpath_str in rpaths_str.split(':') {
                                    let ref rpath = PathBuf::from(rpath_str);
                                    let _rpath;
                                    let rpath = match rpath.strip_prefix("$ORIGIN") {
                                        Ok(p) => {
                                            _rpath = origin_dir.join(p);
                                            &_rpath
                                        },
                                        Err(_) => rpath,
                                    };
                                    let ref rpath = normalize_path(rpath);
                                    println!("rpath: {:?}", rpath);
                                    ctx.add_lpath(rpath);
                                }
                            }
                        }
                        DynamicType::NEEDED => {
                            if let elfkit::dynamic::DynamicContent::String(ref name) = dyn.content {
                                let ref needed = String::from_utf8_lossy(&name.0).into_owned();
                                println!("NEEDED: {}", needed);
                                neededs.push(PathBuf::from(needed));
                            }
                        }
                    }
                }
            }
        }

        for needed in &neededs {
            let mut found = false;
            for lpath in &ctx.lpaths {
                let ref rel_dep = lpath.join(needed);
                let ref dep = with_sysroot(&self.sysroot, rel_dep);
                println!("Checking {:?} in {:?}", needed, lpath);
                if dep.exists() {
                    if deps.insert(rel_dep.clone()) {
                        println!("Found: {:?}", dep);
                        self.find_deps(rel_dep, &mut ctx.clone(), deps);
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                println!("Cannot find: {:?}", needed);
            }
        }

        Ok(())
    }
}

fn with_sysroot(sysroot: &Path, path: &Path) -> PathBuf {
    sysroot.join(
        match path.strip_prefix("/") {
            Ok(rel_path) => rel_path,
            Err(_) => path,
        }
    )
}

fn resolve_link(sysroot: &Path, rel_path: &Path) -> Result<PathBuf, LddError> {
    println!("Resolving link: {:?}", rel_path);
    let ref path = with_sysroot(sysroot, rel_path);
    let stat = symlink_metadata(path).context(path.as_ref())?;
    if stat.file_type().is_symlink() {
        let dst_path = read_link(path).context(path.as_ref())?;
        let ref dst_path = if dst_path.is_relative() {
            if let Some(dir) = rel_path.parent() {
                dir.join(dst_path)
            } else {
                dst_path
            }
        } else {
            dst_path
        };
        resolve_link(sysroot, dst_path)
    } else {
        Ok(path.to_path_buf())
    }
}

fn parse_ld_so_conf(sysroot: &Path, rel_path: &Path)
    -> Result<Vec<PathBuf>, LddError>
{
    println!("ld.so.conf: {:?}", rel_path);
    let mut paths = Vec::new();

    println!("> following link {:?}", rel_path);
    let ref path = resolve_link(sysroot, rel_path)?;
    println!("> opening {:?}", path);
    let f = File::open(path).context(path.as_ref())?;
    println!("> open ok");
    let f = BufReader::new(&f);
    for line in f.lines() {
        let line = line.context(path.as_ref())?;
        let line = line.trim();
        if line.starts_with("#") {
            continue;
        }
        if line == "" {
            continue;
        }

        if line.contains(" ") {
            println!("{}", line);
            // TODO(a-koval) Protect from cyclic including
            if line.starts_with("include ") {
                let ref rel_include_glob = PathBuf::from(
                    line.split(" ").last().unwrap()
                );
                let ref include_glob = with_sysroot(sysroot, rel_include_glob);
                println!("include_glob: {:?}", include_glob);
                for entry in glob(include_glob.to_str().unwrap())
                        .expect("Failed to read glob pattern") {
                    let ref include_path = PathBuf::from(
                        entry.unwrap().to_string_lossy().into_owned()
                    );
                    let ref rel_include_path = include_path
                        .strip_prefix(sysroot).unwrap();
                    println!("include_path: {:?}", include_path);
                    paths.extend(parse_ld_so_conf(sysroot, rel_include_path)?);
                }
            }
        } else {
            paths.push(PathBuf::from(line.to_owned()));
        }
    }
    Ok(paths)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut parts = vec!();
    let mut has_root = false;
    let mut prefix = None;
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => { prefix = Some(p); },
            Component::RootDir => { has_root = true; },
            Component::CurDir => {},
            Component::ParentDir => { parts.pop(); },
            Component::Normal(p) => { parts.push(p); },
        }
    }
    let root_part = if let Some(prefix) = prefix {
        prefix.as_os_str().to_os_string()
    } else {
        if has_root { OsString::from(MAIN_SEPARATOR.to_string()) } else { OsString::from("") }
    };
    let mut normalized_path = PathBuf::from(root_part);
    for p in parts {
        normalized_path = normalized_path.join(p);
    }
    normalized_path
}

mod test {
    use std::path::Path;

    use super::normalize_path;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(Path::new("")), Path::new(""));
        assert_eq!(normalize_path(Path::new("/")), Path::new("/"));
        assert_eq!(normalize_path(Path::new(".")), Path::new(""));
        assert_eq!(normalize_path(Path::new("..")), Path::new(""));
        assert_eq!(normalize_path(Path::new("/./..")), Path::new("/"));
        assert_eq!(normalize_path(Path::new("/a/../b/c/./../test.txt")), Path::new("/b/test.txt"));
    }
}