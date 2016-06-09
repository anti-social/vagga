use std::collections::{BTreeMap, HashSet};

use config::Config;
use config::command::MainCommand;
use config::containers::Container;


#[derive(PartialEq, Eq, Hash)]
struct CommandOption<'a> {
    names: &'a [&'a str],
    description: &'a str,
    has_args: bool,
    single: bool,
}

struct BuiltinCommand<'a> {
    name: &'a str,
    description: &'a str,
    accept_container: bool,
    options: &'a [&'a CommandOption<'a>],
}

#[derive(PartialEq, Eq, Hash)]
struct SuperviseOption<'a> {
    opt: &'a CommandOption<'a>,
    accept_children: bool,
}


const NO_IMAGE_DOWNLOAD: &'static CommandOption<'static> =
    &CommandOption
{
    names: &["--no-image-download"],
    description: "Do not download container image from image index",
    has_args: false,
    single: true,
};

const NO_BUILD: &'static CommandOption<'static> = &CommandOption {
    names: &["--no-build"],
    description: "Do not build container even if it is out of date",
    has_args: false,
    single: true,
};

const NO_VERSION_CHECK: &'static CommandOption<'static> =
    &CommandOption
{
    names: &["--no-version-check"],
    description: "Do not check container version",
    has_args: false,
    single: true,
};

const GLOBAL_OPTIONS: &'static [&'static CommandOption<'static>] = &[
    &CommandOption {
        names: &["-V", "--version"],
        description: "Show vagga version and exit",
        has_args: false,
        single: true,
    },
    &CommandOption {
        names: &["-E", "--env", "--environ"],
        description: "Set environment variable for running command",
        has_args: true,
        single: false,
    },
    &CommandOption {
        names: &["-e", "--use-env"],
        description: "Propagate variable VAR into command environment",
        has_args: true,
        single: false,
    },
    &CommandOption {
        names: &["--ignore-owner-check"],
        description: "Ignore checking owner of the project directory",
        has_args: false,
        single: true,
    },
    NO_IMAGE_DOWNLOAD,
    NO_BUILD,
    NO_VERSION_CHECK,
];

const SUPERVISE_OPTIONS:
&'static [&'static SuperviseOption<'static>] = &[
    &SuperviseOption {
        opt: &CommandOption {
            names: &["--only"],
            description: "",
            has_args: true,
            single: true,
        },
        accept_children: true,
    },
    &SuperviseOption {
        opt: &CommandOption {
            names: &["--exclude"],
            description: "",
            has_args: true,
            single: true,
        },
        accept_children: true,
    },
    &SuperviseOption {
        opt: NO_IMAGE_DOWNLOAD,
        accept_children: false,
    },
    &SuperviseOption {
        opt: NO_BUILD,
        accept_children: false,
    },
    &SuperviseOption {
        opt: NO_VERSION_CHECK,
        accept_children: false,
    },
];

const BUILTIN_COMMANDS:
&'static [&'static BuiltinCommand<'static>] = &[
    &BuiltinCommand {
        name: "_build",
        description: "Builds container without running a command",
        accept_container: true,
        options: &[
            &CommandOption {
                names: &["--force"],
                description: "",
                has_args: false,
                single: true,
            },
        ]
    },
    &BuiltinCommand {
        name: "_build_shell",
        description: "",
        accept_container: false,
        options: &[]
    },
    &BuiltinCommand {
        name: "_clean",
        description: "Removes images and temporary files created by vagga",
        accept_container: false,
        options: &[
            &CommandOption {
                names: &["--tmp", "--tmp-folders"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["--old", "--old-containers"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["--unused"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["--transient"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["--global"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["-n", "--dry-run"],
                description: "",
                has_args: false,
                single: true,
            },
        ]
    },
    &BuiltinCommand {
        name: "_create_netns",
        description: "Setups network namespace",
        accept_container: false,
        options: &[
            &CommandOption {
                names: &["--dry-run"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["--no-iptables"],
                description: "",
                has_args: false,
                single: true,
            },
        ]
    },
    &BuiltinCommand {
        name: "_destroy_netns",
        description: "Destroys network namespace",
        accept_container: false,
        options: &[
            &CommandOption {
                names: &["--dry-run"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["--no-iptables"],
                description: "",
                has_args: false,
                single: true,
            },
        ]
    },
    &BuiltinCommand {
        name: "_init_storage_dir",
        description: "",
        accept_container: false,
        options: &[]
    },
    &BuiltinCommand {
        name: "_list",
        description: "List of commands (similar to running vagga without command)",
        accept_container: false,
        options: &[]
    },
    &BuiltinCommand {
        name: "_pack_image",
        description: "Pack image into the tar archive, optionally compressing it",
        accept_container: true,
        options: &[
            &CommandOption {
                names: &["-f", "--file"],
                description: "",
                has_args: true,
                single: true,
            },
            &CommandOption {
                names: &["-z", "--gzip"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["-j", "--bzip2"],
                description: "",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["-J", "--xz"],
                description: "",
                has_args: false,
                single: true,
            },
            NO_IMAGE_DOWNLOAD,
            NO_BUILD,
            NO_VERSION_CHECK,
        ]
    },
    &BuiltinCommand {
        name: "_push_image",
        description: "Push container image into the image cache",
        accept_container: true,
        options: &[
            NO_IMAGE_DOWNLOAD,
            NO_BUILD,
            NO_VERSION_CHECK,
        ]
    },
    &BuiltinCommand {
        name: "_run",
        description: "Runs arbitrary command in container defined in vagga.yaml",
        accept_container: true,
        options: &[
            &CommandOption {
                names: &["-W", "--writable"],
                description: "Create transient writeable container \
                    for running the command",
                has_args: false,
                single: true,
            },
            NO_IMAGE_DOWNLOAD,
            NO_BUILD,
            NO_VERSION_CHECK,
        ]
    },
    &BuiltinCommand {
        name: "_run_in_netns",
        description: "Runs arbitrary command inside network namespace",
        accept_container: true,
        options: &[
            &CommandOption {
                names: &["--pid"],
                description: "Run in the namespace of the process with PID",
                has_args: true,
                single: true,
            },
            NO_IMAGE_DOWNLOAD,
            NO_BUILD,
            NO_VERSION_CHECK,
        ]
    },
    &BuiltinCommand {
        name: "_version_hash",
        description: "Prints version hash for the container",
        accept_container: true,
        options: &[
            &CommandOption {
                names: &["-s", "--short"],
                description: "Print short container version, \
                    like used in directory names 8 chars",
                has_args: false,
                single: true,
            },
            &CommandOption {
                names: &["-fd3"],
                description: "Print into file descriptor #3 instead of stdout",
                has_args: false,
                single: true,
            },
        ]
    },
    &BuiltinCommand {
        name: "_check_overlayfs_support",
        description: "",
        accept_container: false,
        options: &[]
    },
];


/**

Transition table:

            ___________                     _________
           |           |——————————————————>|         |
  +————————| GlobalCmd |———————————+       | UserCmd |
  |  +—————|___________|——————+    |       |_________|
  |  |                        |    |
  |  |     ______________     |    |      ______________
  |  +———>|              |————+    +————>|              |
  |       | GlobalOption |               | SuperviseCmd |<—————+
  |  +————|______________|<———+    +—————|______________|      |
  |  |                        |    |                           |
  |  |    _________________   |    |     _________________     |
  |  |   |                 |  |    +———>|                 |————+
  |  +——>| GlobalOptionArg |——+         | SuperviseOption |
  |      |_________________|       +————|_________________|<———+
  |                                |                           |
  |        ____________            |    ____________________   |
  +——————>|            |           |   |                    |  |
  +———————| BuiltinCmd |<—————+    +——>| SuperviseOptionArg |——+
  |  +————|____________|      |        |____________________|
  |  |                        |
  |  |    _______________     |
  |  +——>|               |————+
  |      | BuiltinOption |
  |  +———|_______________|<———+
  |  |                        |
  |  |   __________________   |
  |  |  |                  |  |
  |  +—>| BuiltinOptionArg |——+
  |     |__________________|
  |
  |       ______________
  |      |              |
  +—————>| ContainerCmd |
         |______________|

*/
enum States<'a> {
    GlobalCmd,
    GlobalOption(&'a CommandOption<'a>),
    GlobalOptionArg(&'a CommandOption<'a>),
    UserCmd(&'a str),
    SuperviseCmd(&'a str),
    SuperviseOption(&'a str, &'a SuperviseOption<'a>),
    SuperviseOptionArg(&'a str, &'a SuperviseOption<'a>),
    BuiltinCmd(&'a BuiltinCommand<'a>),
    BuiltinOption(&'a BuiltinCommand<'a>, &'a CommandOption<'a>),
    BuiltinOptionArg(&'a BuiltinCommand<'a>, &'a CommandOption<'a>),
    ContainerCmd,
}

struct CommandCompletion<'a> {
    name: &'a str,
    description: Option<&'a str>,
}

struct OptionCompletion<'a> {
    names: &'a [&'a str],
    description: Option<&'a str>,
}

enum Completion<'a> {
    Cmd(CommandCompletion<'a>),
    Opt(OptionCompletion<'a>),
}

struct CompletionGroup<'a> {
    name: &'a str,
    completions: Vec<Completion<'a>>,
}

struct CompletionState<'a> {
    commands: &'a BTreeMap<String, MainCommand>,
    containers: &'a BTreeMap<String, Container>,
    state: States<'a>,
    single_global_options: HashSet<&'a CommandOption<'a>>,
    single_command_options: HashSet<&'a CommandOption<'a>>,
    supervise_single_options: HashSet<&'a SuperviseOption<'a>>,
    supervise_chosen_children: HashSet<&'a str>,
}

impl<'a> CompletionState<'a> {
    pub fn new(
        commands: &'a BTreeMap<String, MainCommand>,
        containers: &'a BTreeMap<String, Container>
    ) -> CompletionState<'a> {

        CompletionState {
            commands: commands,
            containers: containers,
            state: States::GlobalCmd,
            single_global_options: HashSet::new(),
            single_command_options: HashSet::new(),
            supervise_single_options: HashSet::new(),
            supervise_chosen_children: HashSet::new(),
        }
    }

    pub fn trans(&mut self, arg: &'a str) {
        let mut next_state: Option<States> = None;
        {
            match self.state {
                States::GlobalCmd |
                States::GlobalOptionArg(_) => {
                    next_state = self.maybe_user_cmd(arg);
                    if let None = next_state {
                        next_state = self.maybe_global_option(arg);
                    }
                    if let None = next_state {
                        next_state = self.maybe_builtin_cmd(arg);
                    }
                },
                States::GlobalOption(opt) => {
                    if opt.has_args {
                        next_state = Some(
                            States::GlobalOptionArg(opt));
                    } else {
                        next_state = self.maybe_user_cmd(arg);
                        if let None = next_state {
                            next_state = self.maybe_global_option(arg);
                        }
                        if let None = next_state {
                            next_state = self.maybe_builtin_cmd(arg);
                        }
                        if let None = next_state {
                            next_state = Some(States::GlobalCmd);
                        }
                    }
                },
                States::UserCmd(_) => {},
                States::SuperviseCmd(cmd_name) |
                States::SuperviseOptionArg(cmd_name, _) => {
                    next_state = self.maybe_supervise_option(
                        arg, cmd_name);
                },
                States::SuperviseOption(cmd_name, opt) => {
                    if opt.opt.has_args {
                        next_state = Some(
                            States::SuperviseOptionArg(cmd_name, opt));
                    } else {
                        next_state = self.maybe_supervise_option(
                            arg, cmd_name);
                        if let None = next_state {
                            next_state = Some(
                                States::SuperviseCmd(cmd_name));
                        }
                    }
                },
                States::BuiltinCmd(cmd) |
                States::BuiltinOptionArg(cmd, _) => {
                    next_state = self.maybe_builtin_option(arg, cmd);
                    if let None = next_state {
                        next_state = Some(States::ContainerCmd);
                    }
                },
                States::BuiltinOption(cmd, opt) => {
                    if opt.has_args {
                        next_state = Some(
                            States::BuiltinOptionArg(cmd, opt));
                    } else {
                        next_state = self.maybe_builtin_option(
                            arg, cmd);
                        if let None = next_state {
                            next_state = Some(States::BuiltinCmd(cmd));
                        }
                    }
                },
                States::ContainerCmd => {},
            }
        }

        if let Some(next_state) = next_state {
            match next_state {
                States::SuperviseOption(_, opt) if opt.opt.single => {
                    self.supervise_single_options.insert(opt);
                },
                States::SuperviseOptionArg(cmd_name, opt) => {
                    if let Some(&MainCommand::Supervise(ref cmd_info)) =
                        self.commands.get(cmd_name)
                    {
                        if opt.accept_children {
                            for (name, child) in cmd_info.children.iter() {
                                if name == arg {
                                    self.supervise_chosen_children.insert(arg);
                                }
                                if child.get_tags().iter().any(|t| t == arg) {
                                    self.supervise_chosen_children.insert(arg);
                                }
                            }
                        }
                    }
                },
                States::GlobalOption(opt) if opt.single => {
                    self.single_global_options.insert(opt);
                },
                States::BuiltinOption(_, opt) if opt.single => {
                    self.single_command_options.insert(opt);
                },
                _ => {},
            }
            self.state = next_state;
        }
    }

    fn maybe_user_cmd(&self, arg: &'a str) -> Option<States<'a>> {
        for (cmd_name, user_cmd) in self.commands.iter() {
            if arg != cmd_name {
                continue;
            }
            match *user_cmd {
                MainCommand::Command(_) => {
                    return Some(States::UserCmd(cmd_name));
                },
                MainCommand::Supervise(_) => {
                    return Some(States::SuperviseCmd(cmd_name));
                },
            }
        }
        return None;
    }

    fn maybe_global_option(&self, arg: &'a str)
        -> Option<States<'a>>
    {
        for opt in GLOBAL_OPTIONS {
            for &opt_name in opt.names {
                if arg == opt_name {
                    return Some(States::GlobalOption(opt));
                }
            }
        }
        return None;
    }

    fn maybe_supervise_option(&self, arg: &'a str, cmd_name: &'a str)
        -> Option<States<'a>>
    {
        for opt in SUPERVISE_OPTIONS {
            for &opt_name in opt.opt.names {
                if arg == opt_name {
                    return Some(
                        States::SuperviseOption(cmd_name, opt));
                }
            }
        }
        return None;
    }

    fn maybe_builtin_cmd(&self, arg: &'a str) -> Option<States<'a>> {
        for cmd in BUILTIN_COMMANDS {
            if cmd.name == arg {
                return Some(States::BuiltinCmd(cmd));
            }
        }
        return None;
    }

    fn maybe_builtin_option(&self, arg: &'a str,
        cmd: &'a BuiltinCommand<'a>)
        -> Option<States<'a>>
    {
        for cmd_opt in cmd.options {
            for &opt_name in cmd_opt.names {
                if arg == opt_name {
                    return Some(States::BuiltinOption(cmd, cmd_opt));
                }
            }
        }
        return None;
    }

    pub fn complete(&self, cur: &str) -> Vec<CompletionGroup> {
        let mut completions: Vec<CompletionGroup> = Vec::new();

        match self.state {
            States::GlobalCmd |
            States::GlobalOptionArg(_) => {
                completions.extend(self.complete_global(cur));
            },
            States::GlobalOption(opt) if !opt.has_args => {
                completions.extend(self.complete_global(cur));
            },
            States::UserCmd(cmd_name) => {
                if let Some(&MainCommand::Command(ref cmd_info)) =
                    self.commands.get(cmd_name)
                {
                    if cmd_info.has_args() {
                        completions.push(
                            CompletionGroup {
                                name: "file",
                                completions: vec!(),
                            }
                        );
                    }
                }
            },
            States::SuperviseCmd(_) => {
                completions.push(
                    self.get_supervise_options_completion_group());
            },
            States::SuperviseOption(cmd_name, opt) |
            States::SuperviseOptionArg(cmd_name, opt) => {
                completions.push(
                    self.get_supervise_children_completion_group(
                        cmd_name, opt));
                if cur.starts_with("-") || !opt.opt.has_args {
                    completions.push(
                        self.get_supervise_options_completion_group());
                }
            },
            States::BuiltinCmd(cmd) |
            States::BuiltinOptionArg(cmd, _) => {
                completions = self.complete_builtin(cur, cmd);
            },
            States::BuiltinOption(cmd, opt) if !opt.has_args => {
                completions = self.complete_builtin(cur, cmd);
            },
            _ => {},
        }
        // completions.retain(|c| c.starts_with(cur));
        return completions;
    }

    fn complete_global(&self, cur: &str)
        -> Vec<CompletionGroup>
    {
        let mut completions: Vec<CompletionGroup> = Vec::new();

        completions.push(self.get_user_completion_group());
        if cur.starts_with("_") {
            completions.push(
                self.get_builtin_completion_group());
        }
        // if cur.starts_with("-") {
            completions.push(
                self.get_global_options_completion_group());
        // }

        return completions;
    }

    fn complete_builtin(&self, cur: &str, cmd: &BuiltinCommand<'a>)
        -> Vec<CompletionGroup>
    {
        let mut completions = Vec::new();

        if cmd.accept_container {
            completions.push(self.get_containers_completion_group());
        }
        if cur.starts_with("-") {
            completions.push(
                self.get_command_options_completion_group(cmd));
        }

        return completions;
    }

    fn get_user_completion_group(&self)
        -> CompletionGroup
    {
        let mut completions = Vec::new();

        for (name, command) in self.commands.iter() {
            let description = match *command {
                MainCommand::Command(ref cmd) => cmd.description.as_ref(),
                MainCommand::Supervise(ref cmd) => cmd.description.as_ref(),
            };
            completions.push(
                Completion::Cmd(
                    CommandCompletion {
                        name: name,
                        description: description.map(|d| &d[..]),
                    }
                )
            );
        }

        return CompletionGroup {
            name: "user command",
            completions: completions,
        };
    }

    fn get_builtin_completion_group(&self)
        -> CompletionGroup
    {
        let mut completions = Vec::new();

        for cmd in BUILTIN_COMMANDS {
            completions.push(
                Completion::Cmd(
                    CommandCompletion {
                        name: cmd.name,
                        description: if cmd.description == "" {
                            None
                        } else {
                            Some(cmd.description)
                        },
                    }
                )
            );
        }

        return CompletionGroup {
            name: "builtin command",
            completions: completions,
        };
    }

    fn get_global_options_completion_group(&self)
        -> CompletionGroup
    {
        let mut completions = Vec::new();

        for opt in GLOBAL_OPTIONS {
            if !self.single_global_options.contains(opt) {
                completions.push(
                    Completion::Opt(
                        OptionCompletion {
                            names: opt.names,
                            description: if opt.description == "" {
                                None
                            } else {
                                Some(opt.description)
                            }
                        }
                    )
                );
            }
        }

        return CompletionGroup {
            name: "global option",
            completions: completions,
        };
    }

    fn get_supervise_options_completion_group(&self)
        -> CompletionGroup
    {
        let mut completions = Vec::new();

        for supervise_opt in SUPERVISE_OPTIONS {
            let opt = supervise_opt.opt;
            if !self.single_global_options.contains(opt) {
                completions.push(
                    Completion::Opt(
                        OptionCompletion {
                            names: opt.names,
                            description: if opt.description == "" {
                                None
                            } else {
                                Some(opt.description)
                            }
                        }
                    )
                );
            }
        }

        return CompletionGroup {
            name: "supervise option",
            completions: completions,
        };
    }

    fn get_supervise_children_completion_group(&self,
        cmd_name: &'a str, opt: &SuperviseOption<'a>)
        -> CompletionGroup
    {
        let mut completions = Vec::new();

        if let Some(&MainCommand::Supervise(ref cmd_info)) =
            self.commands.get(cmd_name)
        {
            if opt.accept_children {
                for (name, child) in cmd_info.children.iter() {
                    let child_name = &name[..];
                    if !self.supervise_chosen_children.contains(child_name) {
                        let description = child.get_description();
                        completions.push(
                            Completion::Cmd(
                                CommandCompletion {
                                    name: child_name,
                                    description: description.map(|d| &d[..]),
                                }
                            )
                        );
                    }
                    for tag in child.get_tags().iter() {
                        let tag = &tag[..];
                        if !self.supervise_chosen_children.contains(tag) {
                            completions.push(
                                Completion::Cmd(
                                    CommandCompletion {
                                        name: tag,
                                        description: None,
                                    }
                                )
                            );
                        }
                    }
                }
            }
        }

        return CompletionGroup {
            name: "supervise child",
            completions: completions,
        };
    }

    fn get_containers_completion_group(&self)
        -> CompletionGroup
    {
        let mut completions = Vec::new();

        for name in self.containers.keys() {
            completions.push(
                Completion::Cmd(
                    CommandCompletion {
                        name: name,
                        description: None,
                    }
                )
            );
        }

        return CompletionGroup {
            name: "container",
            completions: completions,
        };
    }

    fn get_command_options_completion_group(&self,
        cmd: &BuiltinCommand<'a>)
        -> CompletionGroup
    {
        let mut completions = Vec::new();

        for opt in cmd.options {
            if !self.single_command_options.contains(opt) {
                completions.push(
                    Completion::Opt(
                        OptionCompletion {
                            names: opt.names,
                            description: if opt.description == "" {
                                None
                            } else {
                                Some(opt.description)
                            }
                        }
                    )
                );
            }
        }

        return CompletionGroup {
            name: "command option",
            completions: completions,
        };
    }
}


pub fn generate_completions(config: &Config, args: Vec<String>)
    -> Result<i32, String>
{
    let default_cur_arg = "".to_string();
    let mut splitted_args = args.splitn(2, |a| a == "--");
    let full_args = match splitted_args.next() {
        Some(a) => a.iter().collect::<Vec<_>>(),
        None => vec!(),
    };
    let cur_arg = match splitted_args.next() {
        Some(a) => a.get(0).unwrap_or(&default_cur_arg),
        None => &default_cur_arg,
    };

    let mut state = CompletionState::new(&config.commands,
        &config.containers);
    for arg in full_args {
        state.trans(arg);
    }
    for comp_group in state.complete(cur_arg) {
        println!("# {}", comp_group.name);
        for comp in comp_group.completions {
            match comp {
                Completion::Cmd(cmd_comp) => {
                    match cmd_comp.description {
                        Some(descr) => println!("{}:{}", cmd_comp.name, descr),
                        None => println!("{}", cmd_comp.name),
                    }
                },
                Completion::Opt(opt_comp) => {
                    for name in opt_comp.names {
                        match opt_comp.description {
                            Some(descr) => println!("{}[{}]", name, descr),
                                // descr.replace("[", "\\[").replace("]", "\\]")),
                            None => println!("{}", name),
                        }
                    }
                },
            }
        }
    }

    Ok(0)
}


// pub fn zsh_completions(config: &Config, args: Vec<String>)
//     -> Result<i32, String>
// {
//     let default_cur_arg = "".to_string();
//     let mut splitted_args = args.splitn(2, |a| a == "--");
//     let full_args = match splitted_args.next() {
//         Some(a) => a.iter().collect::<Vec<_>>(),
//         None => vec!(),
//     };
//     let cur_arg = match splitted_args.next() {
//         Some(a) => a.get(0).unwrap_or(&default_cur_arg),
//         None => &default_cur_arg,
//     };

//     let mut state = CompletionState::new(&config.commands,
//         &config.containers);
//     for arg in full_args {
//         state.trans(arg);
//     }
//     for comp in state.complete(cur_arg) {
//         println!("{}", comp);
//     }

//     Ok(0)
// }
