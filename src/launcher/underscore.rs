use std::fs::read_dir;
use std::io::{stdout, stderr};

use argparse::{ArgumentParser};
use argparse::{StoreTrue, List, StoreOption, Store};
use unshare::{Command, Namespace};

use options::build_mode::build_mode;
use options::version_hash;
use container::nsutil::{set_namespace};
use process_util::{run_and_wait, convert_status};
use file_util::human_size;

use super::network;
use super::build::{build_container};
use super::wrap::Wrapper;
use container::util::{version_from_symlink, hardlink_identical_files};
use container::util::{write_container_signature, check_signature};
use launcher::Context;
use launcher::volumes::prepare_volumes;


pub fn run_command(context: &Context, mut args: Vec<String>)
    -> Result<i32, String>
{
    let mut cmdargs = Vec::<String>::new();
    let mut container = "".to_string();
    let mut command = "".to_string();
    let mut copy = false;
    let mut bmode = context.build_mode;
    {
        args.insert(0, "vagga _run".to_string());
        let mut ap = ArgumentParser::new();
        ap.set_description("
            Runs arbitrary command inside the container
            ");
        ap.refer(&mut copy)
            .add_option(&["-W", "--writeable"], StoreTrue,
                "Create translient writeable container for running the command.
                 Currently we use hard-linked copy of the container, so it's
                 dangerous for some operations. Still it's ok for installing
                 packages or similar tasks");
        build_mode(&mut ap, &mut bmode);
        ap.refer(&mut container)
            .add_argument("container", Store,
                "Container to run command in")
            .required();
        ap.refer(&mut command)
            .add_argument("command", Store,
                "Command to run inside the container")
            .required();
        ap.refer(&mut cmdargs)
            .add_argument("arg", List, "Arguments to the command");

        ap.stop_on_first_argument(true);
        match ap.parse(args.clone(), &mut stdout(), &mut stderr()) {
            Ok(()) => {}
            Err(0) => return Ok(0),
            Err(_) => {
                return Ok(122);
            }
        }
    }
    let cinfo = context.config.get_container(&container)?;
    let ver = build_container(context, &container, bmode, false)?;
    prepare_volumes(cinfo.volumes.values(), context)?;

    if context.isolate_network {
        try_msg!(network::isolate_network(),
            "Cannot setup isolated network: {err}");
    }

    let mut cmd: Command = Wrapper::new(Some(&ver), &context.settings);
    cmd.workdir(&context.workdir);
    cmd.arg("_run");
    cmd.args(&args[1..]);
    cmd.map_users_for(cinfo, &context.settings)?;
    cmd.gid(0);
    cmd.groups(Vec::new());
    let res = run_and_wait(&mut cmd).map(convert_status);

    if copy {
        let mut cmd: Command = Wrapper::new(None, &context.settings);
        cmd.workdir(&context.workdir);  // TODO(tailhook) why is it needed?
        cmd.max_uidmap();
        cmd.gid(0);
        cmd.groups(Vec::new());
        cmd.arg("_clean").arg("--transient");
        match cmd.status() {
            Ok(s) if s.success() => {}
            Ok(s) => warn!("The `vagga _clean --transient` {}", s),
            Err(e) => warn!("Failed to run `vagga _clean --transient`: {}", e),
        }

    }
    return res;
}

pub fn run_in_netns(context: &Context, cname: String, mut args: Vec<String>)
    -> Result<i32, String>
{
    let mut cmdargs: Vec<String> = vec!();
    let mut container = "".to_string();
    let mut pid = None;
    let mut bmode = context.build_mode;
    {
        args.insert(0, "vagga ".to_string() + &cname);
        let mut ap = ArgumentParser::new();
        ap.set_description(
            "Run command (or shell) in one of the vagga's network namespaces");
        ap.refer(&mut pid)
            .add_option(&["--pid"], StoreOption, "
                Run in the namespace of the process with PID.
                By default you get shell in the \"gateway\" namespace.
                ");
        build_mode(&mut ap, &mut bmode);
        ap.refer(&mut container)
            .add_argument("container", Store,
                "Container to run command in")
            .required();
        ap.refer(&mut cmdargs)
            .add_argument("command", List,
                "Command (with arguments) to run inside container")
            .required();

        ap.stop_on_first_argument(true);
        match ap.parse(args, &mut stdout(), &mut stderr()) {
            Ok(()) => {}
            Err(0) => return Ok(0),
            Err(_) => {
                return Ok(122);
            }
        }
    }
    let ver = build_container(context, &container, bmode, false)?;
    network::join_gateway_namespaces()?;
    if let Some::<i32>(pid) = pid {
        set_namespace(format!("/proc/{}/ns/net", pid), Namespace::Net)
            .map_err(|e| format!("Error setting networkns: {}", e))?;
    }
    let mut cmd: Command = Wrapper::new(Some(&ver), &context.settings);
    cmd.workdir(&context.workdir);
    cmd.arg(cname);
    cmd.arg(container.clone());
    cmd.args(&cmdargs);
    run_and_wait(&mut cmd).map(convert_status)
}

pub fn version_hash(ctx: &Context, cname: &str, mut args: Vec<String>)
    -> Result<i32, String>
{
    args.insert(0, "vagga _version_hash".to_string());
    let opt = match version_hash::Options::parse(&args, false) {
        Ok(x) => x,
        Err(e) => return Ok(e),
    };
    let mut cmd: Command = Wrapper::new(None, &ctx.settings);
    cmd.workdir(&ctx.workdir);
    cmd.map_users_for(
        ctx.config.get_container(&opt.container)?,
        &ctx.settings)?;
    cmd.gid(0);
    cmd.groups(Vec::new());
    cmd.arg(&cname).args(&args[1..]);
    cmd.status()
    .map(convert_status)
    .map_err(|e| format!("Error running `vagga_wrapper {}`: {}",
                         cname, e))
}

pub fn hardlink_containers(ctx: &Context, mut args: Vec<String>)
    -> Result<i32, String>
{
    let mut global = false;
    args.insert(0, "vagga _hardlink".to_string());
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Indexes and hardlinks containers");
        ap.refer(&mut global)
            .add_option(&["--global"], StoreTrue,
                        "Hardlink containers between projects.");
        ap.stop_on_first_argument(true);
        match ap.parse(args.clone(), &mut stdout(), &mut stderr()) {
            Ok(()) => {},
            Err(0) => return Ok(0),
            Err(_) => return Ok(122),
        }
    }

    let vagga_dir = ctx.config_dir.join(".vagga");
    // let ver = version_from_symlink(vagga_dir.join(&container))?;

    // let roots_dir = vagga_dir.join(".roots");
    // let cont_dir = roots_dir.join(&ver);
    // if !cont_dir.join("index.ds1").exists() {
    //     write_container_signature(&cont_dir)?;
    // }

    let roots_dirs = if let Some(ref storage_dir) = ctx.ext_settings.storage_dir {
        warn!("Storage dir is: {:?}", storage_dir);
        let mut roots_dirs = vec!();
        for entry in try_msg!(read_dir(storage_dir),
                              "Error reading directory: {err}")
        {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.starts_with(".") {
                        continue;
                    }
                    let roots = path.join(".roots");
                    if !roots.exists() {
                        continue;
                    }
                    let index = roots.join("index.ds1");
                    if !index.exists() {
                        write_container_signature(&roots)?;
                    }
                    roots_dirs.push(roots);
                },
                Err(e) => continue,
            }
        }
        roots_dirs
    } else {
        let roots_dir = vagga_dir.join(".roots");
        let mut roots_dirs = vec!();
        for entry in try_msg!(read_dir(&roots_dir),
            "Error reading directory {path:?}: {err}", path=&roots_dir)
        {
            match entry {
                Ok(entry) => {
                    let root_dir = entry.path();
                    if !root_dir.is_dir() {
                        continue;
                    }
                    if root_dir.file_name()
                        .map_or(false, |n| n.to_string_lossy().starts_with("."))
                    {
                        continue;
                    }
                    let index_path = root_dir.join("index.ds1");
                    if !index_path.exists() {
                        warn!("Writing index into {:?}", &root_dir);
                        write_container_signature(&root_dir)?;
                    }
                    roots_dirs.push(root_dir);
                },
                Err(e) => continue,
            }
        }
        roots_dirs
    };

    match hardlink_identical_files(&roots_dirs[..]) {
        Ok((count, size)) => {
            warn!("Found and linked {} files", count);
            Ok(0)
        },
        // Ok((count, size)) => {
        //     warn!("Found and linked {} ({}) identical files \
        //            from other containers", count, human_size(size));
        //     Ok(0)
        // },
        Err(msg) => {
            Err(format!("Error when linking container files: {}", msg))
        },
    }
}

pub fn verify_container(ctx: &Context, mut args: Vec<String>)
    -> Result<i32, String>
{
    args.insert(0, "vagga _verify".to_string());
    let mut container = "".to_string();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Verifies container files checksum");
        ap.refer(&mut container)
            .add_argument("container", Store, "Container to verify");
        ap.stop_on_first_argument(true);
        match ap.parse(args.clone(), &mut stdout(), &mut stderr()) {
            Ok(()) => {},
            Err(0) => return Ok(0),
            Err(_) => return Ok(122),
        }
    }

    let vagga_dir = ctx.config_dir.join(".vagga");
    let ver = version_from_symlink(vagga_dir.join(&container))?;

    let roots_dir = vagga_dir.join(".roots");
    let cont_dir = roots_dir.join(&ver);

    match check_signature(&cont_dir) {
        Ok(None) => Ok(0),
        Ok(Some(ref diff)) => {
            println!("Container was corrupted");
            if !diff.missing_paths.is_empty() {
                println!("Missing paths:");
                for p in &diff.missing_paths {
                    println!("  {}", p.to_string_lossy());
                }
            }
            if !diff.extra_paths.is_empty() {
                println!("Extra paths:");
                for p in &diff.extra_paths {
                    println!("  {}", p.to_string_lossy());
                }
            }
            if !diff.corrupted_paths.is_empty() {
                println!("Corrupted paths:");
                for p in &diff.corrupted_paths {
                    println!("  {}", p.to_string_lossy());
                }
            }
            Ok(1)
        },
        Err(e) => Err(format!("Error checking container signature: {}", e)),
    }
}

pub fn passthrough(ctx: &Context, cname: &str, args: Vec<String>)
    -> Result<i32, String>
{
    let mut cmd: Command = Wrapper::new(None, &ctx.settings);
    cmd.workdir(&ctx.workdir);
    cmd.max_uidmap();
    cmd.gid(0);
    cmd.groups(Vec::new());
    cmd.arg(&cname).args(&args);
    cmd.status()
    .map(convert_status)
    .map_err(|e| format!("Error running `vagga_wrapper {}`: {}",
                         cname, e))
}
