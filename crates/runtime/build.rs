fn main () {
    cc::Build::new()
        .file("src/c/ffi_lib.c")
        .file("src/c/creation.c")
        .compile("ffi_lib");
}
