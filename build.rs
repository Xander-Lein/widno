fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut resource = winres::WindowsResource::new();
        if let Some(toolkit) = std::env::var_os("PATH").and_then(|path| {
            std::env::split_paths(&path).find(|candidate| {
                candidate.join("windres.exe").is_file() && candidate.join("ar.exe").is_file()
            })
        }) {
            resource.set_toolkit_path(toolkit.to_string_lossy().as_ref());
        }
        resource.set_icon("assets/widno-icon.ico");
        resource.compile().expect("embed Widno Windows icon");
        println!(
            "cargo:rustc-link-arg={}",
            std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR"))
                .join("resource.o")
                .display()
        );
    }
}
