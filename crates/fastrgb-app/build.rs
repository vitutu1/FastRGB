//! Embeds the FastRGB multi-resolution `.ico` and Win32 VERSIONINFO resource into `fastrgb.exe`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => return,
    };
    let ico_path = manifest_dir
        .join("..")
        .join("..")
        .join("assets")
        .join("fastrgb.ico");
    println!("cargo:rerun-if-changed={}", ico_path.display());

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if !ico_path.is_file() {
        return;
    }

    let out_dir = match env::var("OUT_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => return,
    };
    let rc_file = out_dir.join("fastrgb.rc");
    let res_file = out_dir.join("fastrgb.res");

    let ico_escaped = ico_path.to_string_lossy().replace('\\', "\\\\");
    let rc_content = format!(
        r#"1 ICON "{ico_escaped}"
1 VERSIONINFO
FILEVERSION 1,0,1,0
PRODUCTVERSION 1,0,1,0
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "FastRGB"
            VALUE "FileDescription", "FastRGB - Native RGB Studio"
            VALUE "FileVersion", "1.0.1.0"
            VALUE "InternalName", "fastrgb"
            VALUE "OriginalFilename", "fastrgb.exe"
            VALUE "ProductName", "FastRGB"
            VALUE "ProductVersion", "1.0.1"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    );

    if fs::write(&rc_file, rc_content).is_err() {
        return;
    }

    let Some(rc_exe) = find_rc_exe() else {
        println!("cargo:warning=rc.exe not found; skipping Windows icon embedding");
        return;
    };

    let status = Command::new(rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_file)
        .arg(&rc_file)
        .status();

    if let Ok(st) = status
        && st.success()
        && res_file.is_file()
    {
        println!("cargo:rustc-link-arg-bins={}", res_file.display());
    }
}

fn find_rc_exe() -> Option<PathBuf> {
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }
    let kits_bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let entries = fs::read_dir(kits_bin).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path().join("x64").join("rc.exe"))
        .filter(|p| p.is_file())
        .collect();
    candidates.sort();
    candidates.pop()
}
