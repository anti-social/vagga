setup() {
    cd /work/tests/inheritance
}

#@test "inheritance: Run bc" {
#    run vagga calc 100*24
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "2400" ]]
#    link=$(readlink .vagga/calc)
#    [[ $link = ".roots/calc.5af5e7a3/root" ]]
#}
#
#@test "inheritance: Inherit from container with deep structure" {
#    run vagga deep-cat
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "world" ]]
#    link=$(readlink .vagga/sub)
#    [[ $link = ".roots/sub.9f9d0b57/root" ]]
#}
#
#@test "inheritance: Test hardlink copy of the deep structure" {
#    run vagga deep-cat-copy
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "world" ]]
#}
#
#@test "inheritance: Build mount" {
#    run vagga hello-mount
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "Hello World!" ]]
#    link=$(readlink .vagga/hellomount)
#    [[ $link = ".roots/hellomount.9c7c2e59/root" ]]
#}
#
#@test "inheritance: Build copy from mount" {
#    run vagga hello-copy-from-mount
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "Hello World!" ]]
#    link=$(readlink .vagga/hellocopyfrommount)
#    [[ $link = ".roots/hellocopyfrommount.70fc36a1/root" ]]
#}
#
#@test "inheritance: Build copy" {
#    run vagga hello-copy
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "Hello World!" ]]
#    link=$(readlink .vagga/hellocopy)
#    [[ $link = ".roots/hellocopy.0ce5ff73/root" ]]
#}
#
#@test "inheritance: Build copy contenthash" {
#    run vagga hello-copy-ch
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "Hello World!" ]]
#    link=$(readlink .vagga/hellocopy-contenthash)
#    [[ $link = ".roots/hellocopy-contenthash.96121dc4/root" ]]
#}
#
#@test "inheritance: Build copy file" {
#    run vagga hello-copy-file
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "Hello World!" ]]
#    link=$(readlink .vagga/hellocopyfile)
#    [[ $link = ".roots/hellocopyfile.0ce5ff73/root" ]]
#}
#
#@test "inheritance: Build copy file with contenthash" {
#    run vagga hello-copy-file-ch
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "Hello World!" ]]
#    link=$(readlink .vagga/hellocopyfile-contenthash)
#    [[ $link = ".roots/hellocopyfile-contenthash.96121dc4/root" ]]
#}
#
#@test "inheritance: Deep inheritance" {
#    run vagga ok
#    printf "%s\n" "${lines[@]}"
#    [[ $status -eq 0 ]]
#    [[ ${lines[${#lines[@]}-1]} = "10" ]]
#    link=$(readlink .vagga/c10)
#    [[ $link = ".roots/c10.e264500e/root" ]]
#}

@test "lddcopy" {
    run vagga _build minimum-container
    printf "%s\n" "${lines[@]}"
    [[ $status = 0 ]]
    run vagga _run minimum-container bc < <(echo 1+2)
    printf "%s\n" "${lines[@]}"
    [[ $status = 0 ]]
    [[ $output = "3" ]]
}
