//! Windows build settings of the dedicated server: `sjk-server.exe` carries
//! SJK's icon (`assets/branding/sjk.ico`, which Explorer and shortcuts show)
//! and names itself "Sol JK dedicated server" in its version strings, which
//! Task Manager shows. Other platforms have no executable icons.

/// SJK's icon set, relative to this crate.
const ICON: &str = "../../assets/branding/sjk.ico";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(ICON)
        .set("ProductName", "Sol JK")
        .set("FileDescription", "Sol JK dedicated server")
        .set("OriginalFilename", "sjk-server.exe");
    // A missing resource compiler costs the icon, not the build.
    if let Err(error) = resource.compile() {
        println!("cargo:warning=sjk-server is built without its icon: {error}");
    }
}
