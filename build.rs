fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("assets/widno-icon.ico");
        resource.compile().expect("embed Widno Windows icon");
    }
}
