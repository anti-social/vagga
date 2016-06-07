#compdef vagga

_vagga() {
    local curcontext=$curcontext
    # typeset -A opt_args

    local rc=1

    _vagga_opts_global=(
        '--environ[Environ option]'
        '-e[Environ option]'
    )
    _test_commands=(
        'test:Test command'
        'test-another:Another test command'
    )
    _builtin_commands=(
        '_build:Builds container without running a command'
        '_build_shell'
        '_check_overlayfs_support'
        '_clean:Removes images and temporary files created by vagga'
        '_create_netns:Setups network namespace'
        '_destroy_netns:Destroys network namespace'
        '_init_storage_dir'
        '_list:List of commands (similar to running vagga without command)'
        '_run:Run arbitrary command'
    )

    local -a user_commands
    local -a builtin_commands
    local -a containers
    local -a options

    # echo
    # echo "state: $state"
    # echo $_vagga_opts_global[@]
    # echo ${words[@]}

    # cur="${COMP_WORDS[COMP_CWORD]}"
    # COMPREPLY=( $(vagga _compgen "${COMP_WORDS[@]:1:$((COMP_CWORD-1))}" -- ${cur} 2>/dev/null) )
    # ARGS=( $(./vagga _compgen -- $1 2>/dev/null) )
    # IFS=$'\r\n' GLOBIGNORE='*' eval 'ARGS=( $(/home/alexk/projects/vagga/compgen.sh) )'
    # echo "${words[@]}"
    # echo $CURRENT
    # echo "${words[2,CURRENT-1]}"
    # echo "${words[CURRENT]}"
    # echo "${words[@]:1:$((CURRENT-1))} -- ${words[$CURRENT]}"
    # echo "/home/alexk/projects/vagga/vagga _compgen ${words[@]:1:$((CURRENT-1))} -- ${words[$CURRENT]}"
    # echo "==="

    # _path_files -W .vagga/rust-musl
    # compadd -f -- tar gz untar
    
    IFS=$'\r\n' GLOBIGNORE='*' eval 'ARGS=( $(/home/alexk/projects/vagga/vagga _compgen ${words[1,CURRENT-1]} -- ${words[CURRENT]}) )'
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
        elif [[ "${comp}" == "# "* ]] && [[ "${comp}" == *" option" ]]; then
            group_name="options"
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
        elif [[ "${group_name}" = "options" ]]; then
            options+=("${comp}")
        fi
    done

    if [[ ${#user_commands[@]} != 0 ]]; then
        _describe -t user-commands 'user command' user_commands && rc=0
    fi
    if [[ ${#builtin_commands[@]} != 0 ]]; then
        _describe -t builtin-commands 'builtin command' builtin_commands && rc=0
    fi
    if [[ ${#containers[@]} != 0 ]]; then
        _describe -t containers 'container' containers && rc=0
    fi
    if [[ ${#options[@]} != 0 ]]; then
        _arguments ${options[@]} && rc=0
    fi

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

    return rc
}

_vagga
