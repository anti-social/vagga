use std::path::{Path, PathBuf};
use std::fs::File;
use std::os::unix::fs::{PermissionsExt, MetadataExt};

use libmount::{BindMount, Remount};

use elfkit::Elf;

use quire::validate as V;
use quick_error::ResultExt;
use config::read_config;
use config::containers::Container as Cont;
use version::short_version;
use container::util::{copy_dir};
use file_util::{Dir, ShallowCopy, ensure_symlink};
use build_step::{BuildStep, VersionError, StepError, Digest, Config, Guard};

use builder::error::StepError as E;
use builder::dns::revert_name_files;
use builder::commands::copy::{create_path_filter, hash_path};
use builder::commands::copy::{hash_file_content};

// Build Steps
#[derive(Debug, Deserialize)]
pub struct Container(String);

impl Container {
    pub fn config() -> V::Scalar {
        V::Scalar::new()
    }
}

#[derive(Deserialize, Debug)]
pub struct Build {
    pub container: String,
    pub source: PathBuf,
    pub path: Option<PathBuf>,
    pub temporary_mount: Option<PathBuf>,
    pub content_hash: bool,
}

impl Build {
    pub fn config() -> V::Structure<'static> {
        V::Structure::new()
        .member("container", V::Scalar::new())
        .member("source".to_string(),
            V::Directory::new().absolute(true).default("/"))
        .member("path".to_string(),
            V::Directory::new().absolute(true).optional())
        .member("temporary_mount".to_string(),
            V::Directory::new().absolute(true).optional())
        .member("content_hash", V::Scalar::new().default(false))
    }
}


#[derive(Deserialize, Debug)]
pub struct GitSource {
    pub url: String,
    pub revision: Option<String>,
    pub branch: Option<String>,
}

#[derive(Deserialize, Debug)]
pub enum Source {
    Git(GitSource),
    Container(String),
    Directory,
}

#[derive(Deserialize, Debug)]
pub struct SubConfig {
    pub source: Source,
    pub path: PathBuf,
    pub container: String,
    pub cache: Option<bool>,
}

impl SubConfig {
    pub fn config() -> V::Structure<'static> {
        V::Structure::new()
        .member("source", V::Enum::new()
            .option("Directory", V::Nothing)
            .option("Container", V::Scalar::new())
            .option("Git", V::Structure::new()
                .member("url", V::Scalar::new())
                .member("revision", V::Scalar::new().optional())
                .member("branch", V::Scalar::new().optional()))
            .optional()
            .default_tag("Directory"))
        .member("path".to_string(), V::Directory::new()
            .absolute(false)
            .default("vagga.yaml"))
        .member("container", V::Scalar::new())
        .member("cache", V::Scalar::new().optional())
    }
}

#[derive(Deserialize, Debug)]
pub struct LddCopy {
    pub libraries_from_container: String,
    pub binaries: Vec<PathBuf>,
}

impl LddCopy {
    pub fn config() -> V::Structure<'static> {
        V::Structure::new()
            .member("libraries_from_container", V::Scalar::new())
            .member("binaries", V::Sequence::new(V::Scalar::new()))
    }
}

pub fn build(binfo: &Build, guard: &mut Guard, build: bool)
    -> Result<(), StepError>
{
    let ref name = binfo.container;
    let cont = guard.ctx.config.containers.get(name)
        .expect("Subcontainer not found");  // TODO
    if build {
        let version = short_version(&cont, &guard.ctx.config)
            .map_err(|(s, e)| format!("step {}: {}", s, e))?;
        let container = Path::new("/vagga/base/.roots")
            .join(format!("{}.{}", name, version));
        let path = container.join("root")
            .join(binfo.source.strip_prefix("/").unwrap());

        // Update container use when using it as subcontainer (fixes #267)
        File::create(Path::new(&container).join("last_use"))
            .map_err(|e| warn!("Can't write image usage info: {}", e)).ok();

        if let Some(ref dest_rel) = binfo.path {
            let dest = Path::new("/vagga/root")
                .join(dest_rel.strip_prefix("/").unwrap());
            if path.is_dir() {
                try_msg!(copy_dir(&path, &dest, None, None),
                    "Error copying dir {p:?}: {err}", p=path);
            } else {
                try_msg!(ShallowCopy::new(&path, &dest).copy(),
                    "Error copying file {p:?}: {err}", p=path);
            }
        } else if let Some(ref dest_rel) = binfo.temporary_mount {
            let dest = Path::new("/vagga/root")
                .join(dest_rel.strip_prefix("/").unwrap());
            try_msg!(Dir::new(&dest).create(),
                "Error creating destination dir: {err}");
            BindMount::new(&path, &dest).mount()?;
            Remount::new(&dest).bind(true).readonly(true).remount()?;
            guard.ctx.mounted.push(dest);
        }
    }
    Ok(())
}

fn real_copy(name: &String, cont: &Cont, guard: &mut Guard)
    -> Result<(), StepError>
{
    let version = short_version(&cont, &guard.ctx.config)
        .map_err(|(s, e)| format!("step {}: {}", s, e))?;
    let container = format!("/vagga/base/.roots/{}.{}", name, version);

    // Update container use when using it as subcontainer (fixes #267)
    File::create(Path::new(&container).join("last_use"))
        .map_err(|e| warn!("Can't write image usage info: {}", e)).ok();

    let root = Path::new(&container).join("root");
    try_msg!(copy_dir(&root, &Path::new("/vagga/root"),
                      None, None),
        "Error copying dir {p:?}: {err}", p=root);
    Ok(())
}

pub fn clone(name: &String, guard: &mut Guard, build: bool)
    -> Result<(), StepError>
{
    let cont = guard.ctx.config.containers.get(name)
        .expect("Subcontainer not found");  // TODO
    for b in cont.setup.iter() {
        b.build(guard, false)
            .map_err(|e| E::SubStep(b.0.clone(), Box::new(e)))?;
    }
    if build {
        real_copy(name, cont, guard)?;
    }
    Ok(())
}

fn find_config(cfg: &SubConfig, guard: &mut Guard)
    -> Result<Config, StepError>
{
    let path = match cfg.source {
        Source::Container(ref container) => {
            let cont = guard.ctx.config.containers.get(container)
                .expect("Subcontainer not found");  // TODO
            let version = short_version(&cont, &guard.ctx.config)
                .map_err(|(s, e)| format!("step {}: {}", s, e))?;
            let container = Path::new("/vagga/base/.roots")
                .join(format!("{}.{}", container, version));

            // Update container use when using it as subcontainer (fixes #267)
            File::create(Path::new(&container).join("last_use"))
                .map_err(|e| warn!("Can't write image usage info: {}", e))
                .ok();

            container.join("root").join(&cfg.path)
        }
        Source::Git(ref _git) => {
            unimplemented!();
        }
        Source::Directory => {
            Path::new("/work").join(&cfg.path)
        }
    };
    Ok(read_config(&path.parent().expect("parent exists"), Some(&path), true)?)
}

pub fn subconfig(cfg: &SubConfig, guard: &mut Guard, build: bool)
    -> Result<(), StepError>
{
    let subcfg = find_config(cfg, guard)?;
    let cont = subcfg.containers.get(&cfg.container)
        .expect("Subcontainer not found");  // TODO
    for b in cont.setup.iter() {
        b.build(guard, build)
            .map_err(|e| E::SubStep(b.0.clone(), Box::new(e)))?;
    }
    Ok(())
}

impl BuildStep for Container {
    fn name(&self) -> &'static str { "Container" }
    fn hash(&self, cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        let cont = cfg.containers.get(&self.0)
            .ok_or(VersionError::ContainerNotFound(self.0.to_string()))?;
        for b in cont.setup.iter() {
            debug!("Versioning setup: {:?}", b);
            hash.command(b.name());
            b.hash(cfg, hash)?;
        }
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        clone(&self.0, guard, build)?;
        revert_name_files()?;
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        Some(&self.0)
    }
}
impl BuildStep for Build {
    fn name(&self) -> &'static str { "Build" }
    fn hash(&self, cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        let cinfo = cfg.containers.get(&self.container)
            .ok_or(VersionError::ContainerNotFound(self.container.clone()))?;
        if self.content_hash {
            let version = short_version(&cinfo, cfg)?;
            let root = Path::new("/vagga/base/.roots")
                .join(format!("{}.{}", self.container, version))
                .join("root");
            if !root.exists() {
                return Err(VersionError::New);
            }
            if let Some(ref dest_rel) = self.path {
                let filter = create_path_filter(&Vec::new(), Some(true),
                    &None, &None, false)?;
                let spath = self.source.strip_prefix("/")
                    .expect("absolute_source_path");
                hash_path(hash, &root.join(&spath), &filter, |h, p, st| {
                    h.field("filename", p);
                    h.field("mode", st.permissions().mode() & 0o7777);
                    h.field("uid", st.uid());
                    h.field("gid", st.gid());
                    hash_file_content(h, p, st)
                        .map_err(|e| VersionError::Io(e, PathBuf::from(p)))?;
                    Ok(())
                })?;
                hash.field("path", dest_rel);
            } else if let Some(_) = self.temporary_mount {
                unimplemented!("Build: combination of \
                    content-hash and temporary-mount are not supported yet");
            }
        } else {
            for b in cinfo.setup.iter() {
                debug!("Versioning setup: {:?}", b);
                hash.command(b.name());
                b.hash(cfg, hash)?;
            }
            // TODO(tailhook) should we hash our params?!?!
        }
        Ok(())
    }
    fn build(&self, guard: &mut Guard, do_build: bool)
        -> Result<(), StepError>
    {
        build(&self, guard, do_build)
    }
    fn is_dependent_on(&self) -> Option<&str> {
        Some(&self.container)
    }
}
impl BuildStep for SubConfig {
    fn name(&self) -> &'static str { "SubConfig" }
    fn hash(&self, cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        let path = match self.source {
            Source::Container(ref container) => {
                let cinfo = cfg.containers.get(container)
                    .ok_or(VersionError::ContainerNotFound(container.clone()))?;
                let version = short_version(&cinfo, cfg)?;
                Path::new("/vagga/base/.roots")
                    .join(format!("{}.{}", container, version))
                    .join("root").join(&self.path)
            }
            Source::Git(ref _git) => {
                unimplemented!();
            }
            Source::Directory => {
                Path::new("/work").join(&self.path)
            }
        };
        if !path.exists() {
            return Err(VersionError::New);
        }
        let subcfg = read_config(
            path.parent().expect("has parent directory"),
            Some(&path), true)?;
        let cont = subcfg.containers.get(&self.container)
            .ok_or(VersionError::ContainerNotFound(self.container.to_string()))?;
        for b in cont.setup.iter() {
            debug!("Versioning setup: {:?}", b);
            hash.command(b.name());
            b.hash(cfg, hash)?;
        }
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        subconfig(self, guard, build)?;
        revert_name_files()?;
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        match self.source {
            Source::Directory => None,
            Source::Container(ref name) => Some(name),
            Source::Git(ref _git) => None,
        }
    }
}

impl BuildStep for LddCopy {
    fn name(&self) -> &'static str { "LddCopy" }
    fn hash(&self, cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool) -> Result<(), StepError> {
        let ref container_name = self.libraries_from_container;
        let container = guard.ctx.config.containers.get(container_name)
            .expect("Subcontainer not found");  // TODO
        let version = short_version(&container, &guard.ctx.config)
            .map_err(|(s, e)| format!("step {}: {}", s, e))?;
        let ref container_path = Path::new("/vagga/base/.roots")
            .join(format!("{}.{}", container_name, version));
        println!("Container path: {:?}", container_path);
        if !build {
            return Ok(());
        }
        for bin in &self.binaries {
            copy_binary(&container_path.join("root"), bin)?;
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        Some(&self.libraries_from_container)
    }
}

fn copy_binary(container_root: &Path, bin: &Path) -> Result<(), StepError> {
    use elfkit;

    let ref src = match bin.strip_prefix("/") {
        Ok(stripped_bin) => container_root.join(&stripped_bin),
        Err(_) => Path::new("/work").join(&bin)
    };
    let deps = find_binary_deps(src)?;
//    deps.push("ld-musl-x86_64.so.1".to_string());
    for dep in &deps {
        copy_dependency(container_root, dep)?;
    }
    let ref dst = Path::new("/vagga/root/usr/bin")
        .join(bin.file_name().unwrap());
    try_msg!(Dir::new("/vagga/root/usr/bin").recursive(true).create(),
                "Error creating destination directory: {err}");
    println!("Copying {:?} -> {:?}", src, dst);
    let mut cp = ShallowCopy::new(src, dst);
    cp.copy().context((src, dst))?;
    Ok(())
}

fn find_binary_deps(bin: &Path) -> Result<Vec<String>, StepError> {
    let mut src_file = File::open(src).unwrap();
    let mut src_elf = Elf::from_reader(&mut src_file).unwrap();
    let mut deps = Vec::new();
    for shndx in 0..src_elf.sections.len() {
        if src_elf.sections[shndx].header.shtype == elfkit::types::SectionType::DYNAMIC {
            src_elf.load(shndx, &mut src_file).unwrap();
            let dynamic = src_elf.sections[shndx].content.as_dynamic().unwrap();

            for dyn in dynamic.iter() {
                //                if dyn.dhtype == elfkit::types::DynamicType::RPATH {
                //                    if let elfkit::dynamic::DynamicContent::String(ref name) = dyn.content {
                //                        self.lpaths.push(join_paths(
                //                            &self.sysroot, &String::from_utf8_lossy(&name.0).into_owned()))
                //                    }
                //                }
                if dyn.dhtype == elfkit::types::DynamicType::NEEDED {
                    if let elfkit::dynamic::DynamicContent::String(ref name) = dyn.content {
                        //                        println!("Dep: {}", String::from_utf8_lossy(&name.0));
                        deps.push(String::from_utf8_lossy(&name.0).into_owned());
                    }
                }
            }
        }
    }
    Ok(deps)
}

fn copy_dependency(sysroot: &Path, dep: String) -> Result<(), StepError> {
    let dst_sysroot = Path::new("/vagga/root");
    let lpaths = &[Path::new("lib"), Path::new("usr/lib")];
    for lpath in lpaths {
        let ref src_lib = lpath.join(dep);
        println!("Checking {:?}", src_lib);
        match sysroot.join(src_lib).symlink_metadata() {
            Some(src_stat) => {
                println!("Dep exists: {:?}", src_lib);
                _copy_dependency(sysroot, dst_sysroot, src_lib);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                continue
            }
            Err(e) => {
                return Err(e);
            }
        };

//        if src_lib.exists() {
//            println!("Dep exists: {:?}", src_lib);
//            let ref dst_dir = Path::new("/vagga/root").join(lpath);
//            let ref dst_lib = dst_dir.join(dep);
//            try_msg!(Dir::new(dst_dir).recursive(true).create(),
//                "Error creating directory: {err}");
//            let mut cp = ShallowCopy::new(src_lib, dst_lib);
//            cp.copy().context((src_lib, dst_lib))?;
//        }
    }
    Ok(())
}

fn _copy_dependency(src_sysroot: &Path, dst_sysroot: &Path, rel_path: &Path) -> Result<(), StepError> {
    use std::fs;

    let src_path = src_sysroot.join(rel_path);
    let dst_path = dst_sysroot.join(rel_path);
    let src_stat = src_path.symlink_metadata().unwrap();
    if src_stat.file_type().is_symlink() {
        let ref tgt_path = fs::read_link(src_lib).unwrap();
        let ref tgt_lib = match tgt_path.strip_prefix("/") {
            Ok(stripped_tgt) => src_sysroot.join(stripped_tgt),
            Err(_) => sysroot.join(tgt_path),
        };
        ensure_symlink(tgt_path, dst_path).unwrap();
        _copy_dependency(src_sysroot, dst_sysroot, tgt_path)?;
    } else if src_stat.is_file() {
        let mut cp = ShallowCopy::new(src_lib, dst_lib);
        cp.copy().context((src_lib, dst_lib))?;
    }

    let ref dst_lib = dst_dir.join(dep);
    try_msg!(Dir::new(dst_dir).recursive(true).create(),
                    "Error creating directory: {err}");
    if stat.file_type().is_symlink() {
        copy_symlink(sysroot, src_lib).unwrap();
    } else if stat.is_file() {
        let mut cp = ShallowCopy::new(src_lib, dst_lib);
        cp.copy().context((src_lib, dst_lib))?;
    }

    copy_symlink();
    Ok(())
}