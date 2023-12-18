fn main() {
    // fix for static linking issue (undefined references to 'deflate')
    // caused by wrong order of -lz -png (defining group with correct order
    // fixes that)
    #[cfg(target_os="linux")]
    println!("cargo:rustc-link-arg=-Wl,--start-group,-lpng,-lz,--end-group");
}
