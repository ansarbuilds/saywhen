fn main() {
    #[cfg(target_os = "macos")]
    {
        // whisper-rs-sys links `-lclang_rt.osx` when MACOSX_DEPLOYMENT_TARGET is set,
        // but hardcodes `/Applications/Xcode.app/...` which fails on Macs that only have
        // CommandLineTools (`/Library/Developer/CommandLineTools/...`). Ask `cc` for its
        // active resource directory and add `<resource_dir>/lib/darwin` to the linker path.
        if let Ok(output) = std::process::Command::new("cc")
            .arg("-print-resource-dir")
            .output()
        {
            if output.status.success() {
                let resource_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let darwin_lib_dir = std::path::Path::new(&resource_dir).join("lib/darwin");
                if darwin_lib_dir.exists() {
                    println!(
                        "cargo:rustc-link-search=native={}",
                        darwin_lib_dir.display()
                    );
                }
            }
        }
    }

    tauri_build::build()
}
