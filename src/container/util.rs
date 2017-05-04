use std::fs::File;
use std::fs::{read_dir, remove_file, remove_dir, rename};
use std::fs::{symlink_metadata, read_link, hard_link};
use std::io::{self, BufReader};
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dir_signature::v1::{Entry, EntryKind, Parser};
use dir_signature::v1::merge::FileMergeBuilder;
use itertools::Itertools;
use libc::{uid_t, gid_t};

use super::root::temporary_change_root;
use file_util::{Dir, shallow_copy};

quick_error!{
    #[derive(Debug)]
    pub enum CopyDirError {
        ReadDir(path: PathBuf, err: io::Error) {
            display("Can't read dir {:?}: {}", path, err)
        }
        Stat(path: PathBuf, err: io::Error) {
            display("Can't stat {:?}: {}", path, err)
        }
        CopyFile(src: PathBuf, dst: PathBuf, err: io::Error) {
            display("Can't copy {:?} -> {:?}: {}", src, dst, err)
        }
        CreateDir(path: PathBuf, err: io::Error) {
            display("Can't create dir {:?}: {}", path, err)
        }
        ReadLink(path: PathBuf, err: io::Error) {
            display("Can't read symlink {:?}: {}", path, err)
        }
        Symlink(path: PathBuf, err: io::Error) {
            display("Can't create symlink {:?}: {}", path, err)
        }
    }
}

pub fn clean_dir<P: AsRef<Path>>(dir: P, remove_dir_itself: bool) -> Result<(), String> {
    _clean_dir(dir.as_ref(), remove_dir_itself)
}

fn _clean_dir(dir: &Path, remove_dir_itself: bool) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    // We temporarily change root, so that symlinks inside the dir
    // would do no harm. But note that dir itself can be a symlink
    temporary_change_root::<_, _, _, String>(dir, || {
        let mut path = PathBuf::from("/");
        let diriter = try_msg!(read_dir(&path),
             "Can't read directory {d:?}: {err}", d=dir);
        let mut stack = vec![diriter];
        'next_dir: while let Some(mut diriter) = stack.pop() {
            while let Some(entry) = diriter.next() {
                let entry = try_msg!(entry, "Error reading dir entry: {err}");
                let typ = try_msg!(entry.file_type(),
                    "Can't stat {p:?}: {err}", p=entry.path());
                path.push(entry.file_name());
                if typ.is_dir() {
                    stack.push(diriter);  // push directory back to stack
                    let niter = read_dir(&path)
                         .map_err(|e| format!("Can't read directory {:?}: {}",
                                              dir, e))?;
                    stack.push(niter);  // push new directory to stack
                    continue 'next_dir;
                } else {
                    try_msg!(remove_file(&path),
                        "Can't remove file {dir:?}: {err}", dir=entry.path());
                    path.pop();
                }
            }
            if Path::new(&path) == Path::new("/") {
                break;
            } else {
                try_msg!(remove_dir(&path),
                    "Can't remove dir {p:?}: {err}", p=path);
                path.pop();
            }
        }
        Ok(())
    })?;
    if remove_dir_itself {
        try_msg!(remove_dir(dir),
            "Can't remove dir {dir:?}: {err}", dir=dir);
    }
    return Ok(());
}

pub fn copy_dir(old: &Path, new: &Path,
    owner_uid: Option<uid_t>, owner_gid: Option<gid_t>)
    -> Result<(), CopyDirError>
{
    use self::CopyDirError::*;
    // TODO(tailhook) use reflinks if supported
    let dir = read_dir(old).map_err(|e| ReadDir(old.to_path_buf(), e))?;
    let mut stack = vec![dir];
    let mut oldp = old.to_path_buf();
    let mut newp = new.to_path_buf();
    'next_dir: while let Some(mut dir) = stack.pop() {
        while let Some(item) = dir.next() {
            let entry = item.map_err(|e| ReadDir(old.to_path_buf(), e))?;
            let filename = entry.file_name();
            oldp.push(&filename);
            newp.push(&filename);

            let oldp_stat = oldp.symlink_metadata()
                .map_err(|e| Stat(oldp.clone(), e))?;
            let copy_rc = shallow_copy(&oldp, &oldp_stat, &newp,
                    owner_uid, owner_gid, None)
                .map_err(|e| CopyFile(oldp.clone(), newp.clone(), e))?;
            if !copy_rc {
                stack.push(dir);  // Return dir to stack
                let ndir = read_dir(&oldp)
                    .map_err(|e| ReadDir(oldp.to_path_buf(), e))?;
                stack.push(ndir); // Add new dir to the stack too
                continue 'next_dir;
            }
            oldp.pop();
            newp.pop();
        }
        oldp.pop();
        newp.pop();
    }
    Ok(())
}

pub fn hardlink_dir(old: &Path, new: &Path) -> Result<(), CopyDirError> {
    use self::CopyDirError::*;
    // TODO(tailhook) use reflinks if supported
    let dir = read_dir(old).map_err(|e| ReadDir(old.to_path_buf(), e))?;
    let mut stack = vec![dir];
    let mut oldp = old.to_path_buf();
    let mut newp = new.to_path_buf();
    'next_dir: while let Some(mut dir) = stack.pop() {
        while let Some(item) = dir.next() {
            let entry = item.map_err(|e| ReadDir(old.to_path_buf(), e))?;
            let filename = entry.file_name();
            oldp.push(&filename);
            newp.push(&filename);

            let typ = entry.file_type()
                .map_err(|e| Stat(oldp.clone(), e))?;
            if typ.is_file() {
                hard_link(&oldp, &newp)
                    .map_err(|e| CopyFile(oldp.clone(), newp.clone(), e))?;
            } else if typ.is_dir() {
                let stat = symlink_metadata(&oldp)
                    .map_err(|e| Stat(oldp.clone(), e))?;
                if !newp.is_dir() {
                    Dir::new(&newp)
                            .mode(stat.mode())
                            .uid(stat.uid())
                            .gid(stat.gid())
                            .create()
                        .map_err(|e| CreateDir(newp.clone(), e))?;
                }
                stack.push(dir);  // Return dir to stack
                let ndir = read_dir(&oldp)
                    .map_err(|e| ReadDir(oldp.to_path_buf(), e))?;
                stack.push(ndir); // Add new dir to the stack too
                continue 'next_dir;
            } else if typ.is_symlink() {
                let lnk = read_link(&oldp)
                               .map_err(|e| ReadLink(oldp.clone(), e))?;
                symlink(&lnk, &newp)
                    .map_err(|e| Symlink(newp.clone(), e))?;
            } else {
                warn!("Unknown file type {:?}", &entry.path());
            }
            oldp.pop();
            newp.pop();
        }
        oldp.pop();
        newp.pop();
    }
    Ok(())
}

pub fn version_from_symlink<P: AsRef<Path>>(path: P) -> Result<String, String>
{
    let lnk = path.as_ref();
    let path = read_link(&path)
        .map_err(|e| format!("Can't read link {:?}: {}", lnk, e))?;
    path.iter().rev().nth(1).and_then(|x| x.to_str())
    .ok_or_else(|| format!("Bad symlink {:?}: {:?}", lnk, path))
    .map(|x| x.to_string())
}

#[cfg(feature="containers")]
pub fn find_and_link_identical_files(
    container_name: &str, cont_ver: &str, cont_dir: &Path, roots_dir: &Path)
    -> Result<(u32, u64), String>
{
    let container_root = cont_dir.join("root");
    let main_ds_path = cont_dir.join("index.ds1");
    if !main_ds_path.exists() {
        warn!("No index file exists. Can't hardlink");
        return Ok((0, 0));
    }
    let main_ds_reader = BufReader::new(try_msg!(File::open(&main_ds_path),
        "Error opening file {path:?}: {err}", path=&main_ds_path));
    let mut main_ds_parser = try_msg!(Parser::new(main_ds_reader),
        "Error parsing signature file: {err}");

    let _paths_names_times = get_container_paths_names_times(
        roots_dir, &roots_dir.join(cont_ver))?;
    let mut paths_names_times = _paths_names_times.iter()
        .map(|&(ref p, ref n, ref t)| (p, n, t))
        .collect::<Vec<_>>();
    // Sort by current container name equality
    // then by container name and then by modification date
    paths_names_times.sort_by_key(|&(_, n, t)| {
        (n == container_name, n, t)
    });
    let mut merged_ds_builder = FileMergeBuilder::new();
    for (_, cont_group) in paths_names_times
        .into_iter()
        .rev()
        .group_by(|&(_, n, _)| n)
        .into_iter()
    {
        for (cont_path, _, _) in cont_group.take(5) {
            merged_ds_builder.add(&cont_path.join("root"),
                                  &cont_path.join("index.ds1"));
        }
    }
    let mut merged_ds = try_msg!(merged_ds_builder.finalize(),
        "Error parsing signature files: {err}");
    let mut merged_ds_iter = merged_ds.iter();

    let tmp = cont_dir.join(".link.tmp");
    let mut count = 0;
    let mut size = 0;
    for entry in main_ds_parser.iter() {
        match entry {
            Ok(Entry::File{
                path: ref lnk_path,
                exe: lnk_exe,
                size: lnk_size,
                hashes: ref lnk_hashes,
            }) => {
                let lnk = container_root.join(
                    match lnk_path.strip_prefix("/") {
                        Ok(lnk_path) => lnk_path,
                        Err(_) => continue,
                    });
                let lnk_stat = lnk.symlink_metadata().map_err(|e|
                    format!("Error querying file stats: {}", e))?;
                for tgt_entry in merged_ds_iter
                    .advance(&EntryKind::File(lnk_path))
                {
                    match tgt_entry {
                        (tgt_base_path,
                         Ok(Entry::File{
                             path: ref tgt_path,
                             exe: tgt_exe,
                             size: tgt_size,
                             hashes: ref tgt_hashes}))
                            if lnk_exe == tgt_exe &&
                            lnk_size == tgt_size &&
                            lnk_hashes == tgt_hashes =>
                        {
                            let tgt = tgt_base_path.join(
                                match tgt_path.strip_prefix("/") {
                                    Ok(path) => path,
                                    Err(_) => continue,
                                });
                            let tgt_stat = tgt.symlink_metadata().map_err(|e|
                                format!("Error querying file stats: {}", e))?;
                            if lnk_stat.mode() != tgt_stat.mode() ||
                                lnk_stat.uid() != tgt_stat.uid() ||
                                lnk_stat.gid() != lnk_stat.gid()
                            {
                                continue;
                            }
                            if let Err(_) = hard_link(&tgt, &tmp) {
                                remove_file(&tmp).map_err(|e|
                                    format!("Error removing file after failed \
                                             hard linking: {}", e))?;
                                continue;
                            }
                            if let Err(_) = rename(&tmp, &lnk) {
                                remove_file(&tmp).map_err(|e|
                                    format!("Error removing file after failed \
                                             renaming: {}", e))?;
                                continue;
                            }
                            count += 1;
                            size += tgt_size;
                            break;
                        },
                        _ => continue,
                    }
                }
            },
            _ => {},
        }
    }

    Ok((count, size))
}

#[cfg(not(feature="containers"))]
pub fn find_and_link_identical_files(
    container_name: &str, cont_ver: &str, cont_dir: &Path, roots_dir: &Path)
    -> Result<(u32, u64), String>
{
    unimplemented!();
}

fn get_container_paths_names_times(roots_dir: &Path, exclude_path: &Path)
    -> Result<Vec<(PathBuf, String, SystemTime)>, String>
{
    Ok(try_msg!(read_dir(roots_dir),
                "Error reading directory: {err}")
        .filter_map(|x| x.ok())
        .map(|x| x.path())
        .filter(|p| {
            p != exclude_path &&
                p.is_dir() &&
                p.join("index.ds1").is_file()
        })
        .filter_map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_string())
                .map(|n| (p, n))
        })
        .filter(|&(_, ref d)| !d.starts_with("."))
        .filter_map(|(p, d)| {
            let mut dir_name_parts = d.rsplitn(2, '.');
            dir_name_parts.next();
            dir_name_parts.next()
                .map(|n| (p, n.to_string()))
        })
        .filter_map(|(p, n)| {
            p.metadata()
                .and_then(|m| m.modified()).ok()
                .map(|t| (p, n, t))
        })
        .collect::<Vec<_>>())
}
