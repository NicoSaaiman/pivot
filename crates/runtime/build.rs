fn main () {
    cc::Build::new()
        .file("src/c/ffi_lib.c")
        .compile("ffi_lib");
}
