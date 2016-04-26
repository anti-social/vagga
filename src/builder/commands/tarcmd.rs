use std::fs::{File, Permissions};
use std::fs::{create_dir_all, hard_link, set_permissions};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use tar::Archive;
use flate2::read::GzDecoder;
use bzip2::read::BzDecoder;
use xz2::read::XzDecoder;

use builder::context::Context;
use builder::download::{maybe_download_and_check_hashsum};
use builder::commands::generic::run_command_at;
use file_util::{read_visible_entries, create_dir};
use path_util::ToRelative;
use build_step::{BuildStep, VersionError, StepError, Digest, Config, Guard};


#[derive(RustcDecodable, Debug)]
pub struct Tar {
    pub url: String,
    pub sha256: Option<String>,
    pub path: PathBuf,
    pub subdir: PathBuf,
}

#[derive(RustcDecodable, Debug)]
pub struct TarInstall {
    pub url: String,
    pub sha256: Option<String>,
    pub subdir: Option<PathBuf>,
    pub script: String,
}

fn get_entry_file_path(entry_path: &Path, tgt: &Path,
    subdir: Option<&Path>, include: &[&Path], exclude: &[&Path])
    -> Result<(Option<PathBuf>, bool), String>
{
    if include.len() > 0 {
        for include_path in include {
            if !entry_path.starts_with(include_path) {
                return Ok((None, false));
            }
        }
    }
    for exclude_path in exclude {
        if entry_path.starts_with(exclude_path) {
            return Ok((None, false));
        }
    }
    match subdir {
        Some(subdir) => {
            if entry_path.starts_with(subdir) {
                let file_path = tgt.join(try_msg!(entry_path.strip_prefix(subdir), "{err}"));
                Ok((Some(file_path), true))
            } else {
                Ok((None, false))
            }
        },
        None => {
            let file_path = tgt.join(&entry_path);
            Ok((Some(file_path), false))
        },
    }
}

struct Link {
    pub src: PathBuf,
    pub dst: PathBuf,
}

pub fn unpack_file(src: &Path, tgt: &Path, subdir: Option<&Path>,
    include: &[&Path], exclude: &[&Path])
    -> Result<(), String>
{
    info!("Unpacking {} -> {}", src.display(), tgt.display());

    let mut file = try_msg!(File::open(src),
        "Cannot open file {filename:?}: {err}", filename=src);
    let (mut gz, mut bz, mut xz);
    let r: &mut Read = match src.extension().and_then(|x| x.to_str()) {
        Some("gz")|Some("tgz") => {
            gz = try_msg!(GzDecoder::new(file), "Cannot decode file: {err}");
            &mut gz
        }
        Some("bz")|Some("tbz") => {
            bz = BzDecoder::new(file);
            &mut bz
        }
        Some("xz")|Some("txz") => {
            xz = XzDecoder::new(file);
            &mut xz
        }
        _ => {
            &mut file
        }
    };

    try_msg!(create_dir(&tgt, true), "Error creating dir: {err}");

    let mut found_subdir = false;
    let mut tar = Archive::new(r);
    let mut delayed_links = vec!();
    for entry in tar.entries().unwrap() {
        let entry = &mut entry.unwrap();
        let entry_path = entry.header().path().unwrap().to_path_buf();
        if !entry.header().entry_type().is_hard_link() {
            let (file_path, is_inside_subdir) = try!(get_entry_file_path(
                &entry_path, tgt, subdir, include, exclude));
            if !found_subdir {
                found_subdir = is_inside_subdir;
            }
            if let Some(file_path) = file_path {
                debug!("Unpacking {:?}: {:?} => {:?}",
                    entry.header().entry_type(), &entry_path, &file_path);
                try_msg!(entry.unpack(file_path),
                    "Cannot unpack archive: {err}");
            }
        } else {
            // TODO: process case when hard link is inside subdir
            // but destination file is not
            let (file_path, _) = try!(get_entry_file_path(
                &entry_path, tgt, subdir, include, exclude));
            if let Some(file_path) = file_path {
                let link_name = match try_msg!(entry.link_name(),
                    "Error when getting link name: {err}")
                {
                    Some(name) => name,
                    None => return Err(format!("Missing name for hard link")),
                };
                let (link_path, _) = try!(get_entry_file_path(
                    &link_name.into_owned(), tgt, subdir, include, exclude));
                if let Some(link_path) = link_path {
                    let link = Link {
                        src: link_path,
                        dst: file_path,
                    };
                    delayed_links.push(link);
                }
            }
        }
    }
    for link in delayed_links {
        info!("Delayed link: {:?} -> {:?}", link.src, link.dst);
        try_msg!(hard_link(&link.src, &link.dst),
            "Cannot create hard link: {err}");
    }

    match subdir {
        Some(subdir) if !found_subdir => {
            Err(format!("{:?} is not found in archive", subdir))
        },
        _ => Ok(()),
    }
}

pub fn tar_command(ctx: &mut Context, tar: &Tar)
    -> Result<(), String>
{
    let fpath = PathBuf::from("/vagga/root").join(tar.path.rel());
    let filename = try!(maybe_download_and_check_hashsum(
        ctx, &tar.url, tar.sha256.clone()));

    if &tar.subdir == &Path::new("") || &tar.subdir == &Path::new(".") {
        try!(unpack_file(&filename, &fpath, None, &[], &[]));
    } else {
        try!(unpack_file(&filename, &fpath, Some(&tar.subdir),
            &[&tar.subdir], &[]));
    };
    Ok(())
}

pub fn tar_install(ctx: &mut Context, tar: &TarInstall)
    -> Result<(), String>
{
    let filename = try!(maybe_download_and_check_hashsum(
        ctx, &tar.url, tar.sha256.clone()));

    let tmppath = PathBuf::from("/vagga/root/tmp")
        .join(filename.file_name().unwrap());
    try!(create_dir_all(&tmppath)
         .map_err(|e| format!("Error making dir: {}", e)));
    try!(set_permissions(&tmppath, Permissions::from_mode(0o755))
         .map_err(|e| format!("Error setting permissions: {}", e)));
    try!(unpack_file(&filename, &tmppath, None, &[], &[]));
    let workdir = if let Some(ref subpath) = tar.subdir {
        tmppath.join(subpath)
    } else {
        let items = try!(read_visible_entries(&tmppath)
            .map_err(|e| format!("Error reading dir: {}", e)));
        if items.len() != 1 {
            if items.len() == 0 {
                return Err("Tar archive was empty".to_string());
            } else {
                return Err("Multiple directories was unpacked. \
                    If thats expected use `subdir: \".\"` or any \
                    other directory".to_string());
            }
        }
        items.into_iter().next().unwrap()
    };
    let workdir = PathBuf::from("/").join(
        workdir.rel_to(&Path::new("/vagga/root")).unwrap());
    return run_command_at(ctx, &[
        "/bin/sh".to_string(),
        "-exc".to_string(),
        tar.script.to_string()],
        &workdir);
}

impl BuildStep for Tar {
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        if let Some(ref sha) = self.sha256 {
            hash.field("hash", sha);
        } else {
            hash.field("url", &self.url);
        }
        hash.field("path", self.path.as_os_str().as_bytes());
        hash.field("subdir", self.subdir.as_os_str().as_bytes());
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        if build {
            try!(tar_command(&mut guard.ctx, self));
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}

impl BuildStep for TarInstall {
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        if let Some(ref sha) = self.sha256 {
            hash.field("hash", sha);
        } else {
            hash.field("url", &self.url);
        }
        hash.opt_field("subdir",
            &self.subdir.as_ref().map(|x| x.as_os_str().as_bytes()));
        hash.field("script", &self.script);
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        if build {
            try!(tar_install(&mut guard.ctx, self));
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}
