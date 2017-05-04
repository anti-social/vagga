setup() {
    cd /work/tests/hardlink
}

@test "hardlink cmd" {
    run vagga _build hello
    printf "%s\n" "${lines[@]}"
    [[ $status = 0 ]]
    link=$(readlink .vagga/hello)
    [[ $link = ".roots/hello.0ae0aab6/root" ]]

    run vagga _hardlink hello
    printf "%s\n" "${lines[@]}"
    [[ $status = 0 ]]
    [[ $output = *"Found and linked 0"* ]]

    run vagga _build hello-and-bye
    printf "%s\n" "${lines[@]}"
    [[ $status = 0 ]]
    link=$(readlink .vagga/hello-and-bye)
    [[ $link = ".roots/hello-and-bye.84b3175b/root" ]]

    run vagga _hardlink hello-and-bye
    printf "%s\n" "${lines[@]}"
    [[ $status = 0 ]]
    # There are 2 hardlinks because of /etc/resolv.conf
    [[ $output = *"Found and linked 2"* ]]

    [[ $(stat -c "%i" .vagga/hello/etc/hello.txt) = \
        $(stat -c "%i" .vagga/hello-and-bye/etc/hello.txt) ]]

    [[ $(stat -c "%h" .vagga/hello-and-bye/etc/hello.txt) = 2 ]]
    [[ $(stat -c "%h" .vagga/hello-and-bye/etc/bye.txt) = 1 ]]
}
