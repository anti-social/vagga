extern crate elfkit;
extern crate glob;
extern crate priority_queue;
#[macro_use] extern crate quick_error;

use std::collections::{BTreeSet, HashSet};
use std::io::{self, BufReader, BufRead};
use std::ffi::OsString;
use std::fs::{File, read_link, symlink_metadata};
use std::path::{Path, PathBuf, Component, MAIN_SEPARATOR};

use elfkit::Elf;
use elfkit::types::{DynamicType, SectionType};
use elfkit::dynamic::DynamicContent;

use glob::glob;

use priority_queue::PriorityQueue;

use quick_error::ResultExt;

const DEFAULT_LIBRARY_PATHS: &[&str] = &[
    "/usr/local/lib",
    "/usr/lib",
    "/lib",
];

const DEFAULT_PATH_PRIORITY: i32 = 1000;
const LD_SO_CONF_PRIORITY: i32 = 2000;
const RUNPATH_PRIORITY: i32 = 3000;
const LD_LIBRARY_PATH_PRIORITY: i32 = 4000;
const RPATH_PRIORITY: i32 = 5000;

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum LPathSource {
    RPath,
    LdLibraryPath,
    RunPath,
    LdSoConf,
    DefaultPath,
}

#[derive(Debug, Clone)]
struct LPaths {
    paths: PriorityQueue<PathBuf, i32>,
    // Directories listed in the executable's rpath
    rpath_prio: i32,
    // Directories from the LD_LIBRARY_PATH environment variable
    ld_library_path_prio: i32,
    // Directories listed in the executable's runpath
    runpath_prio: i32,
    // Directories from /etc/ld.so.conf
    ld_so_conf_prio: i32,
    // Default system libraries
    default_path_prio: i32,
}

impl LPaths {
    fn new() -> LPaths {
        LPaths {
            paths: PriorityQueue::new(),
            rpath_prio: RPATH_PRIORITY,
            ld_library_path_prio: LD_LIBRARY_PATH_PRIORITY,
            runpath_prio: RUNPATH_PRIORITY,
            ld_so_conf_prio: LD_SO_CONF_PRIORITY,
            default_path_prio: DEFAULT_PATH_PRIORITY,
        }
    }

    fn add(&mut self, path: PathBuf, source: LPathSource) {
        use LPathSource::*;

        let prio = match source {
            RPath => {
                &mut self.rpath_prio
            }
            LdLibraryPath => {
                &mut self.ld_library_path_prio
            }
            RunPath => {
                &mut self.runpath_prio
            }
            LdSoConf => {
                &mut self.ld_so_conf_prio
            }
            DefaultPath => {
                &mut self.default_path_prio
            }
        };
        let should_update = self.paths.get_priority(&path)
            .map_or(true, |x| *prio > *x);
        if should_update {
            self.paths.push(path, *prio);
        }
        *prio -= 1;
    }

    fn into_vec(self) -> Vec<PathBuf> {
        self.paths.into_sorted_vec()
    }
}

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
        NotFound(dep: String) {
            display("Cannot find dependency: {:?}", dep)
        }
        RecursiveLink(path: PathBuf) {
            display("Recursive link: {:?}", path)
        }
    }
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
        let mut lpaths = LPaths::new();
        for lpath in DEFAULT_LIBRARY_PATHS {
            lpaths.add(PathBuf::from(lpath), LPathSource::DefaultPath);
        }
        match resolve_link(&sysroot, ld_so_conf_path) {
            Ok(_) => {
                let ld_so_conf_paths = parse_ld_so_conf(
                    &sysroot, ld_so_conf_path
                )?;
                for lpath in ld_so_conf_paths {
                    lpaths.add(lpath, LPathSource::LdSoConf);
                }
            }
            Err(_) => {}
        }
        Ok(Ldd {
            sysroot,
            lpaths,
        })
    }

    pub fn deps<P: AsRef<Path>>(&self, rel_path: P)
        -> Result<BTreeSet<PathBuf>, LddError>
    {
        let mut deps = BTreeSet::new();
        self.find_deps(rel_path.as_ref(), &mut deps)?;
        Ok(deps)
    }

    fn find_deps(&self, rel_path: &Path, deps: &mut BTreeSet<PathBuf>)
        -> Result<(), LddError>
    {
        println!("Finding deps for: {:?}", rel_path);
        let ref path = resolve_link(&self.sysroot, rel_path)?;
        println!("Real file path: {:?}", path);
        let ref origin = Path::new("/")
            .join(path.parent().unwrap().strip_prefix(&self.sysroot).unwrap())
            .to_string_lossy()
            .into_owned();
        println!("Origin: {:?}", origin);
        let mut src_file = File::open(path).context(path.as_path())?;
        println!("Opened!");
        let mut src_elf = Elf::from_reader(&mut src_file)?;
        let mut neededs = Vec::new();
        let mut lpaths = self.lpaths.clone();
        for shndx in 0..src_elf.sections.len() {
            if src_elf.sections[shndx].header.shtype == SectionType::DYNAMIC {
                src_elf.load(shndx, &mut src_file)?;
                let dynamic = match src_elf.sections[shndx].content.as_dynamic() {
                    Some(dynamic) => dynamic,
                    None => continue,
                };

                for dyn in dynamic.iter() {
                    match dyn.dhtype {
                        ref dhtype @ DynamicType::RUNPATH |
                        ref dhtype @ DynamicType::RPATH => {
                            if let DynamicContent::String(ref name) = dyn.content {
                                let rpaths_str = String::from_utf8_lossy(&name.0).into_owned();
                                println!("{:?}: {}", dhtype, rpaths_str);
                                for rpath_str in rpaths_str.split(':') {
                                    let rpath = PathBuf::from(rpath_str);
                                    let lpath_source = if dhtype == &DynamicType::RUNPATH {
                                        LPathSource::RunPath
                                    } else {
                                        LPathSource::RPath
                                    };
                                    lpaths.add(rpath, lpath_source);
                                }
                            }
                        }
                        DynamicType::NEEDED => {
                            if let DynamicContent::String(ref name) = dyn.content {
                                let needed = String::from_utf8_lossy(&name.0).into_owned();
                                println!("NEEDED: {}", needed);
                                neededs.push(needed);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let lpaths = lpaths.into_vec();
        println!("lpaths: {:?}", &lpaths);
        for needed in &neededs {
            let mut found = false;
            for lpath in &lpaths {
                // TODO(a-koval) Add support for the $LIB & $PLATFORM tokens and NODEFLIB & ORIGIN flag
                let lpath_str = lpath.to_string_lossy();
                println!("lpath: {}", &lpath_str);
                let lpath_str = lpath_str
                    .replace("${ORIGIN}", origin)
                    .replace("$ORIGIN", origin);
                println!("lpath: {}", &lpath_str);
                // TODO(a-koval) Consider to move normalization into resolve_link function
                let ref lpath = normalize_path(&PathBuf::from(lpath_str));
                let ref rel_dep = lpath.join(needed);
                let ref dep = match resolve_link(&self.sysroot, rel_dep) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                println!("Checking {:?} in {:?}: {:?}", needed, lpath, dep);
                if deps.insert(rel_dep.clone()) {
                    println!("Found: {:?}", dep);
                    self.find_deps(rel_dep, deps)?;
                }
                found = true;
            }
            if !found {
                return Err(LddError::NotFound(needed.clone()));
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

pub fn resolve_link(sysroot: &Path, rel_path: &Path)
    -> Result<(PathBuf, Vec<PathBuf>), LddError>
{
    println!("Resolving link: {:?}", rel_path);
    let mut cur_path = rel_path.to_path_buf();
    let mut links = vec!(cur_path.clone());
    let mut seen_paths = HashSet::new();
    seen_paths.insert(cur_path.clone());
    loop {
        let ref path = with_sysroot(sysroot, &cur_path);
        let stat = symlink_metadata(path).context(path.as_ref())?;
        if stat.file_type().is_symlink() {
            let dst_path = read_link(path).context(path.as_ref())?;
            let dst_path = if dst_path.is_relative() {
                if let Some(dir) = rel_path.parent() {
                    dir.join(dst_path)
                } else {
                    dst_path
                }
            } else {
                dst_path
            };
            if seen_paths.contains(&dst_path) {
                return Err(LddError::RecursiveLink(dst_path));
            } else {
                res.push(dst_path.clone());
                seen_paths.insert(dst_path.clone());
            }
            cur_path = dst_path;
        } else {
            return Ok((path.to_path_buf(), links));
        }
    }
}

//pub fn resolve_link(sysroot: &Path, rel_path: &Path) -> Result<PathBuf, LddError> {
//    let paths = resolve_link_all(sysroot, rel_path)?;
//    paths.
//}

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
        if has_root {
            OsString::from(MAIN_SEPARATOR.to_string())
        } else {
            OsString::from("")
        }
    };
    let mut normalized_path = PathBuf::from(root_part);
    for p in parts {
        normalized_path = normalized_path.join(p);
    }
    normalized_path
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use super::{normalize_path, LPaths, LPathSource};

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(Path::new("")), Path::new(""));
        assert_eq!(normalize_path(Path::new("/")), Path::new("/"));
        assert_eq!(normalize_path(Path::new(".")), Path::new(""));
        assert_eq!(normalize_path(Path::new("..")), Path::new(""));
        assert_eq!(normalize_path(Path::new("/./..")), Path::new("/"));
        assert_eq!(normalize_path(Path::new("/a/../b/c/./../test.txt")), Path::new("/b/test.txt"));
    }

    #[test]
    fn test_lpaths() {
        let mut lpaths = LPaths::new();
        lpaths.add(PathBuf::from("/usr/local/lib"), LPathSource::LdLibraryPath);
        lpaths.add(PathBuf::from("/usr/local/lib"), LPathSource::DefaultPath);
        lpaths.add(PathBuf::from("/usr/lib"), LPathSource::DefaultPath);
        lpaths.add(PathBuf::from("/lib"), LPathSource::DefaultPath);
        lpaths.add(PathBuf::from("/opt/lib"), LPathSource::LdSoConf);
        lpaths.add(PathBuf::from("/lib"), LPathSource::RPath);
        assert_eq!(
            lpaths.into_vec(),
            vec!(
                Path::new("/lib"),
                Path::new("/usr/local/lib"),
                Path::new("/opt/lib"),
                Path::new("/usr/lib"),
            )
        );
    }
}