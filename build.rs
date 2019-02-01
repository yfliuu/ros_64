pub extern crate cc;


fn main() {
    cc::Build::new()
        .file("src/bootloader/entry.S")
        .compile("entry");
}