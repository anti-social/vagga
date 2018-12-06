setup() {
    cd /work/tests/docker
}

@test "docker: hello-world" {
    run vagga _build hello
    printf "%s\n" "${lines[@]}"
    [[ $status -eq 0 ]]

    run vagga hello
    printf "%s\n" "${lines[@]}"
    [[ $status -eq 0 ]]
    [[ ${lines[0]} = "Hello from Docker!" ]]
}
