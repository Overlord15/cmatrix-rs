#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/cmatrix.ico");
    // Optional: shows up in the exe's Properties -> Details tab
    res.set("FileDescription", "cmatrix - Matrix rain for the terminal");
    res.set("ProductName", "cmatrix");
    res.set("LegalCopyright", "GPL-3.0-or-later");
    res.compile().expect("failed to embed Windows resources");
}

#[cfg(not(windows))]
fn main() {}