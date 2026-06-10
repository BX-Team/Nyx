fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/brand/icon.ico");
        println!("cargo:rerun-if-changed=assets/brand/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=failed to embed Windows resources: {e}");
        }
    }
}
