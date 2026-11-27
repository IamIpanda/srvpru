use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=WINDBOT_NATIVE_DIR");
    println!("cargo:rerun-if-env-changed=WINDBOT_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=WINDBOT_DOTNET");

    let target: String = env::var("TARGET")?;
    let host: String = env::var("HOST")?;
    let rid: &str = match target.as_str() {
        "x86_64-pc-windows-msvc" => "win-x64",
        "aarch64-pc-windows-msvc" => "win-arm64",
        "x86_64-unknown-linux-gnu" => "linux-x64",
        "aarch64-unknown-linux-gnu" => "linux-arm64",
        "x86_64-unknown-linux-musl" => "linux-musl-x64",
        "aarch64-unknown-linux-musl" => "linux-musl-arm64",
        "x86_64-apple-darwin" => "osx-x64",
        "aarch64-apple-darwin" => "osx-arm64",
        "aarch64-apple-ios" => "ios-arm64",
        "aarch64-apple-ios-sim" => "iossimulator-arm64",
        "x86_64-apple-ios" => "iossimulator-x64",
        "armv7-linux-androideabi" => "linux-bionic-arm",
        "aarch64-linux-android" => "linux-bionic-arm64",
        "x86_64-linux-android" => "linux-bionic-x64",
        _ => return Err(format!("no WindBot NativeAOT build for target {target}").into()),
    };
    let windows: bool = rid.starts_with("win-");
    let apple: bool = target.contains("-apple-");
    let musl: bool = target.contains("musl");

    if windows {
        let target_features: String = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        if !target_features.split(',').any(|feature: &str| feature == "crt-static") {
            return Err("WindBot Windows archives require RUSTFLAGS=-C target-feature=+crt-static".into());
        }
    }
    if apple && !host.contains("-apple-") {
        return Err("Apple targets require a macOS host with Xcode command line tools.".into());
    }

    let output: PathBuf = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let patch_archive: bool = env::var_os("CARGO_FEATURE_PATCH_ARCHIVE").is_some();
    let native: PathBuf = match env::var_os("WINDBOT_NATIVE_DIR") {
        Some(directory) => PathBuf::from(directory),
        None => publish(&output, rid)?,
    };

    let rid_file: PathBuf = native.join("native-rid.txt");
    if rid_file.is_file() && fs::read_to_string(&rid_file)?.trim() != rid {
        return Err(format!("{} does not match {rid}", rid_file.display()).into());
    }

    let library: PathBuf = native.join(if windows { "WindBot.lib" } else { "WindBot.a" });
    if !library.is_file() {
        return Err(format!("missing {}", library.display()).into());
    }

    let bundled: PathBuf = output.join(if windows { "windbot_native.lib" } else { "libwindbot_native.a" });
    fs::copy(&library, &bundled)?;
    if patch_archive && !windows && !apple {
        retain_modules(&bundled)?;
    }
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static:+whole-archive=windbot_native");

    let manifest: String = fs::read_to_string(native.join("native-dependencies.txt"))?;
    for entry in manifest.lines() {
        let normalized: String = entry.trim().replace('\\', "/");
        let line: &str = normalized.as_str();
        if line.is_empty() { continue }
        if line.starts_with("-framework ") && apple {
            let name: &str = line.trim_start_matches("-framework ").trim();
            println!("cargo:rustc-link-lib=framework={name}");
            continue;
        }
        if let Some(name) = line.strip_prefix("-l") {
            if !windows { println!("cargo:rustc-link-lib=dylib={name}"); }
            continue;
        }
        if windows && line.ends_with(".lib") && !line.contains('/') && !line.contains('\\') {
            println!("cargo:rustc-link-lib=dylib={}", &line[..line.len() - 4]);
            continue;
        }
        if musl && line.ends_with("libstdc++compat.a") { continue }
        link_dependency(&native, &output, line, &target, windows)?;
    }

    if apple {
        let sdk_name: &str = if rid == "ios-arm64" { "iphoneos" } else if rid.starts_with("iossimulator-") { "iphonesimulator" } else { "macosx" };
        let sdk: PathBuf = PathBuf::from(String::from_utf8(Command::new("xcrun").args(["--sdk", sdk_name, "--show-sdk-path"]).output()?.stdout)?.trim());
        println!("cargo:rustc-link-search=native={}", sdk.join("usr/lib/swift").display());
        println!("cargo:rustc-link-search=framework={}", sdk.join("System/Library/Frameworks").display());
        if rid.starts_with("osx-") {
            let resource: PathBuf = PathBuf::from(String::from_utf8(Command::new("xcrun").args(["clang", "-print-resource-dir"]).output()?.stdout)?.trim());
            println!("cargo:rustc-link-search=native={}", resource.join("lib/darwin").display());
            println!("cargo:rustc-link-lib=static=clang_rt.osx");
        }
    }
    Ok(())
}

fn link_dependency(native: &Path, output: &Path, line: &str, target: &str, windows: bool) -> Result<(), Box<dyn Error>> {
    let relative: &Path = Path::new(line);
    if !line.starts_with("runtime/") || relative.components().any(|part: Component<'_>| !matches!(part, Component::Normal(_))) {
        return Err(format!("unsupported native dependency {line}").into());
    }
    let source: PathBuf = native.join(relative);
    if !source.is_file() {
        return Err(format!("missing {}", source.display()).into());
    }
    let file_name: &str = relative.file_name().and_then(OsStr::to_str).ok_or("invalid native dependency name")?;
    let stem: &str = file_name.rsplit_once('.').map(|parts: (&str, &str)| parts.0).ok_or("native dependency has no extension")?;

    if file_name.ends_with(".o") || file_name.ends_with(".obj") {
        let name: String = format!("windbot_object_{stem}");
        let archive: PathBuf = output.join(if windows { format!("{name}.lib") } else { format!("lib{name}.a") });
        fs::remove_file(&archive).ok();
        let status: std::process::ExitStatus = if windows {
            let archiver: OsString = windows_archiver()?;
            Command::new(archiver).arg("/NOLOGO").arg(format!("/OUT:{}", archive.display())).arg(&source).status()?
        } else {
            Command::new("ar").arg("crs").arg(&archive).arg(&source).status()?
        };
        if !status.success() {
            return Err(format!("archiving {} failed", source.display()).into());
        }
        println!("cargo:rustc-link-lib=static:+whole-archive={name}");
        return Ok(())
    }

    let name: &str = if windows && file_name.ends_with(".lib") {
        &file_name[..file_name.len() - 4]
    } else if !windows && file_name.starts_with("lib") && file_name.ends_with(".a") {
        &file_name[3..file_name.len() - 2]
    } else {
        return Err(format!("unsupported native dependency {line}").into());
    };
    let destination: PathBuf = output.join(file_name);
    fs::copy(&source, &destination)?;
    if target.contains("musl") && file_name == "libRuntime.WorkstationGC.a" {
        drop_bundled_unwinder(&destination)?;
    }
    if target.contains("linux") && !target.contains("android") {
        println!("cargo:rustc-link-lib=static:-bundle={name}");
    } else {
        println!("cargo:rustc-link-lib=static={name}");
    }
    Ok(())
}

#[allow(dead_code)]
fn retain_modules(archive: &Path) -> Result<(), Box<dyn Error>> {
    let mut bytes: Vec<u8> = fs::read(archive)?;
    if bytes.get(..8) != Some(b"!<arch>\n") {
        return Err(format!("{} is not a regular ar archive", archive.display()).into())
    }
    let mut offset: usize = 8;
    while offset + 60 <= bytes.len() {
        let size: usize = String::from_utf8_lossy(&bytes[offset + 48..offset + 58]).trim().parse()?;
        let start: usize = offset + 60;
        let end: usize = start + size;
        if end > bytes.len() {
            return Err(format!("truncated member in {}", archive.display()).into())
        }
        if bytes.get(start..start + 4) == Some(b"\x7fELF") {
            retain_modules_in_object(&mut bytes[start..end])?;
        }
        offset = end + (size & 1);
    }
    fs::write(archive, &bytes)?;
    Ok(())
}

#[allow(dead_code)]
fn retain_modules_in_object(object: &mut [u8]) -> Result<(), Box<dyn Error>> {
    let class: u8 = object[4];
    if object[5] != 1 {
        return Err("only little-endian ELF objects are supported".into())
    }
    let wide: bool = class == 2;
    let (table, stride, count, names_index): (usize, usize, usize, usize) = if wide {
        (read_number(object, 0x28, 8) as usize, read_number(object, 0x3A, 2) as usize, read_number(object, 0x3C, 2) as usize, read_number(object, 0x3E, 2) as usize)
    } else {
        (read_number(object, 0x20, 4) as usize, read_number(object, 0x2E, 2) as usize, read_number(object, 0x30, 2) as usize, read_number(object, 0x32, 2) as usize)
    };
    let names: usize = read_number(object, table + names_index * stride + if wide { 24 } else { 16 }, if wide { 8 } else { 4 }) as usize;
    for index in 0..count {
        let entry: usize = table + index * stride;
        let start: usize = names + read_number(object, entry, 4) as usize;
        let length: usize = object[start..].iter().position(|byte: &u8| *byte == 0).ok_or("unterminated section name")?;
        if &object[start..start + length] != b"__modules" {
            continue
        }
        let width: usize = if wide { 8 } else { 4 };
        let flags: u64 = read_number(object, entry + 8, width) | 0x200000;
        object[entry + 8..entry + 8 + width].copy_from_slice(&flags.to_le_bytes()[..width]);
    }
    Ok(())
}

#[allow(dead_code)]
fn read_number(bytes: &[u8], offset: usize, width: usize) -> u64 {
    let mut buffer: [u8; 8] = [0; 8];
    buffer[..width].copy_from_slice(&bytes[offset..offset + width]);
    u64::from_le_bytes(buffer)
}

fn drop_bundled_unwinder(archive: &Path) -> Result<(), Box<dyn Error>> {
    let members: std::process::Output = Command::new("ar").arg("t").arg(archive).output()?;
    let listing: String = String::from_utf8(members.stdout)?;
    if !listing.lines().any(|member: &str| member.trim() == "libunwind.cpp.o") {
        return Ok(())
    }
    let status: std::process::ExitStatus = Command::new("ar").arg("d").arg(archive).arg("libunwind.cpp.o").status()?;
    if !status.success() {
        return Err(format!("removing libunwind.cpp.o from {} failed", archive.display()).into())
    }
    Ok(())
}

fn windows_archiver() -> Result<OsString, Box<dyn Error>> {
    if Command::new("llvm-lib").arg("/?").output().is_ok() {
        return Ok(OsString::from("llvm-lib"))
    }
    let installer: PathBuf = PathBuf::from(env::var_os("ProgramFiles(x86)").ok_or("neither llvm-lib nor ProgramFiles(x86) is available")?).join("Microsoft Visual Studio/Installer/vswhere.exe");
    let output: std::process::Output = Command::new(installer).args(["-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-find", "VC/Tools/MSVC/**/bin/Hostx64/x64/lib.exe"]).output()?;
    let paths: String = String::from_utf8(output.stdout)?;
    let path: &str = paths.lines().next().ok_or("cannot locate lib.exe; install the MSVC C++ workload")?;
    Ok(OsString::from(path))
}

fn publish(output: &Path, rid: &str) -> Result<PathBuf, Box<dyn Error>> {
    let source: PathBuf = match env::var_os("WINDBOT_SOURCE_DIR") {
        Some(directory) => PathBuf::from(directory),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("windbot"),
    };
    let project: PathBuf = source.join("WindBot.csproj");
    if !project.is_file() {
        return Err(format!("missing {}; run git submodule update --init or set WINDBOT_NATIVE_DIR", project.display()).into());
    }

    println!("cargo:rerun-if-changed={}", source.display());
    let publish: PathBuf = output.join("native-built");
    fs::create_dir_all(&publish)?;
    let dotnet: OsString = env::var_os("WINDBOT_DOTNET").unwrap_or_else(|| OsString::from("dotnet"));
    let status: std::process::ExitStatus = Command::new(dotnet)
        .arg("publish")
        .arg(&project)
        .args(["-c", "Release", "-r", rid, "-p:NativeLib=Static"])
        .arg("--artifacts-path")
        .arg(output.join("dotnet"))
        .arg("-o")
        .arg(&publish)
        .status()?;
    if !status.success() {
        return Err("dotnet publish failed".into());
    }
    Ok(publish)
}
