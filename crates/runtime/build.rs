fn main () {
    cc::Build::new()
        .file("src/c/ffi_lib.c")
        .file("src/c/creation.c")
        .file("src/c/monitoring.c")
        .file("src/c/deletion.c")
        .compile("ffi_lib");
}
