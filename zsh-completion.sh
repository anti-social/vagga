#compdef vagga

_vagga() {
    local curcontext=$curcontext state line cmd args
    declare -A opt_args

    local rc=1

    local -a user_commands
    local -a builtin_commands
    local -a containers
    local -a global_options
    local -a supervise_options
    local -a command_options
    local complete_files=0

    # local words=("vagga" "_run" "-")
    # local CURRENT=3

    # echo "'${words[@]}'"
    # echo "'${words[CURRENT]}'"
    # echo "'${words[2,CURRENT-1]}'"
    
    # return 0
    
    # echo
    # echo "state: $state"
    # echo "service: $service"
    # echo "'${words[@]}'"
    # echo $CURRENT
    # echo "'${words[@][2,CURRENT-1]}'"
    # echo "'${words[CURRENT]}'"
    # echo "'/home/alexk/projects/vagga/vagga _compgen ${words[2,CURRENT-1]} -- ${words[CURRENT]}'"

    # _path_files -W .vagga/rust-musl
    # compadd -f -- tar gz untar
    
    IFS=$'\r\n' GLOBIGNORE='*' eval 'ARGS=( $(/home/alexk/projects/vagga/vagga _compgen ${words[2,CURRENT-1]} -- ${words[CURRENT]}) )'
    local group_name
    for comp in "${ARGS[@]}"; do
        if [[ "${comp}" = "# user command" ]]; then
            group_name="user_commands"
            continue
        elif [[ "${comp}" = "# builtin command" ]]; then
            group_name="builtin_commands"
            continue
        elif [[ "${comp}" = "# container" ]]; then
            group_name="containers"
            continue
        elif [[ "${comp}" = "# global option" ]]; then
            group_name="global_options"
            continue
        elif [[ "${comp}" = "# supervise option" ]]; then
            group_name="supervise_options"
            continue
        elif [[ "${comp}" = "# command option" ]]; then
            group_name="command_options"
            continue
        elif [[ "${comp}" = "# file" ]]; then
            complete_files=1
            continue
        elif [[ "${comp}" == "# "* ]]; then
            continue
        fi

        if [[ "${group_name}" = "user_commands" ]]; then
            user_commands+=("${comp}")
        elif [[ "${group_name}" = "builtin_commands" ]]; then
            builtin_commands+=("${comp}")
        elif [[ "${group_name}" = "containers" ]]; then
            containers+=("${comp}")
        elif [[ "${group_name}" = "global_options" ]]; then
            global_options+=("${comp}")
        elif [[ "${group_name}" = "supervise_options" ]]; then
            supervise_options+=("${comp}")
        elif [[ "${group_name}" = "command_options" ]]; then
            command_options+=("${comp}")
        fi
    done
    global_options+=(
      '(-): :->cmd'
      '(-)*:: :->args'
    )
    # echo "${#user_commands[@]}"
    # echo "${#builtin_commands[@]}"
    # echo "${#containers[@]}"
    # echo "${#options[@]}"

    # user_commands=(
    #     make:'Build vagga'
    # )
    # options=(
    #     '-W[Create transient writeable container for running the command]'
    # )

    _arguments -C ${global_options[@]} && return
    echo "_arguments: $?"
    echo "state: $state"
    # if [[ $rc = 0 ]]; then
    #     return
    # fi

    case $state in
         (cmd)
             if [[ ${#user_commands[@]} != 0 ]]; then
                 _describe -t user-commands 'user command' user_commands && rc=0
                 # echo "user_commands: $?"
                 # echo ${user_commands}
             fi
             if [[ ${#builtin_commands[@]} != 0 ]]; then
                 _describe -t builtin-commands 'builtin command' builtin_commands && rc=0
                 # echo "builtin_commands: $?"
                 # echo ${builtin_commands}
             fi
             if [[ ${#containers[@]} != 0 ]]; then
                 _describe -t containers 'container' containers && rc=0
                 # echo "containers: $?"
                 # echo ${containers}
             fi
             ;;
         (args)
             if [[ ${#supervise_options[@]} != 0 ]]; then
                 _arguments ${supervise_options[@]} && rc=0
                 # echo "supervise_options: $?"
                 # echo ${options}
             fi
             if [[ ${#command_options[@]} != 0 ]]; then
                 _arguments ${command_options[@]} && rc=0
                 # echo "command_options: $?"
                 # echo ${options}
             fi
             ;;
    esac
    # if [[ ${complete_files} = 1 ]]; then
    #     _files && rc=0
    # fi
    # rc=0

    # IFS=$'\r\n' GLOBIGNORE='*' eval '_commands=( $(/home/alexk/projects/vagga/vagga _compgen) )'
    # IFS=$'\r\n' GLOBIGNORE='*' eval '__builtin_commands=( $(cat builtin_commands.txt) )'
    # unset builtin_commands
    # for comp in "${__builtin_commands[@]}"; do
    #     if [[ "${comp}" == "# "* ]]; then
    #         continue
    #     fi
    #     builtin_commands+=("${comp}")
    # done
    # if [[ ${#builtin_commands[@]} != 0 ]]; then
    #     _describe -t builtin-commands 'builtin command' builtin_commands && rc=0
    # fi
    # IFS=$'\r\n' GLOBIGNORE='*' eval 'builtin_commands=( $(cat builtin_commands.txt) )'
    # _describe -t builtin-commands 'builtin command' builtin_commands && rc=0
    # local commands
    # for comp in "${test_commands[@]}"; do
    #     if [[ "${comp}" == "# "* ]]; then
    #         continue
    #     fi
    #     commands+=("${comp}")
    # done
    # _describe -t test-commands 'test command' _test_commands && rc=0
    # _describe -t builtin-commands 'builtin command' _builtin_commands && rc=0
    # _arguments -C -S "$ARGS[@]" && rc=0
    # _arguments -s : "$ARGS[@]" && rc=0
    # _arguments -s : "$_vagga_opts_global[@]" && rc=0

    # compadd -X 'Test command' test another-test

    # echo "rc: $rc"
    # echo "==="
    return rc
}

_vagga
