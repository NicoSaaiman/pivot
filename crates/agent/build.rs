fn main () {
    cc::Build::new()
        .file("src/ffi_lib.c")
        .compile("ffi_lib");
}
