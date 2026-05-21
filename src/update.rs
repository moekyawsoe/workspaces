use eframe::egui;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable { version: String, release_notes: String },
    Downloading(f32),
    Installing,
    Success,
    Error(String),
}

pub struct Updater {
    status: Arc<Mutex<UpdateStatus>>,
    asset_url: Arc<Mutex<Option<String>>>,
    asset_name: Arc<Mutex<Option<String>>>,
    ctx: egui::Context,
}

impl Updater {
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            status: Arc::new(Mutex::new(UpdateStatus::Idle)),
            asset_url: Arc::new(Mutex::new(None)),
            asset_name: Arc::new(Mutex::new(None)),
            ctx,
        }
    }

    pub fn get_status(&self) -> UpdateStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn check_for_updates(&self) {
        let status = Arc::clone(&self.status);
        let asset_url = Arc::clone(&self.asset_url);
        let asset_name = Arc::clone(&self.asset_name);
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            *status.lock().unwrap() = UpdateStatus::Checking;
            ctx.request_repaint();

            match fetch_latest_release() {
                Ok((version, notes, url, name)) => {
                    let current = env!("CARGO_PKG_VERSION");
                    if version != current && is_newer(&version, current) {
                        *asset_url.lock().unwrap() = Some(url);
                        *asset_name.lock().unwrap() = Some(name);
                        *status.lock().unwrap() = UpdateStatus::UpdateAvailable {
                            version,
                            release_notes: notes,
                        };
                    } else {
                        *status.lock().unwrap() = UpdateStatus::UpToDate;
                    }
                    ctx.request_repaint();
                }
                Err(e) => {
                    *status.lock().unwrap() = UpdateStatus::Error(e.to_string());
                    ctx.request_repaint();
                }
            }
        });
    }

    pub fn install_update(&self) {
        let status = Arc::clone(&self.status);
        let asset_url = Arc::clone(&self.asset_url);
        let asset_name = Arc::clone(&self.asset_name);
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let (url, name) = {
                let url_guard = asset_url.lock().unwrap();
                let name_guard = asset_name.lock().unwrap();
                (url_guard.clone(), name_guard.clone())
            };

            let url = match url {
                Some(u) => u,
                None => {
                    *status.lock().unwrap() =
                        UpdateStatus::Error("No download URL available".into());
                    ctx.request_repaint();
                    return;
                }
            };

            *status.lock().unwrap() = UpdateStatus::Downloading(0.0);
            ctx.request_repaint();

            match download_with_progress(&url, &name.unwrap_or_else(|| "update".into()), &status, &ctx) {
                Ok(download_path) => {
                    *status.lock().unwrap() = UpdateStatus::Installing;
                    ctx.request_repaint();

                    match install_binary(&download_path) {
                        Ok(_) => {
                            *status.lock().unwrap() = UpdateStatus::Success;
                            ctx.request_repaint();
                        }
                        Err(e) => {
                            *status.lock().unwrap() = UpdateStatus::Error(e.to_string());
                            ctx.request_repaint();
                        }
                    }
                }
                Err(e) => {
                    *status.lock().unwrap() = UpdateStatus::Error(e.to_string());
                    ctx.request_repaint();
                }
            }
        });
    }

    pub fn reset(&self) {
        *self.status.lock().unwrap() = UpdateStatus::Idle;
        *self.asset_url.lock().unwrap() = None;
        *self.asset_name.lock().unwrap() = None;
    }
}

fn fetch_latest_release() -> Result<(String, String, String, String), Box<dyn std::error::Error>> {
    use self_update::backends::github::ReleaseList;

    let releases = ReleaseList::configure()
        .repo_owner("moekyawsoe")
        .repo_name("workspaces")
        .build()?
        .fetch()?;

    if releases.is_empty() {
        return Err("No releases found".into());
    }

    let latest = releases.first().ok_or("No releases found")?;

    let version = latest.version.clone();
    let notes = latest.body.clone().unwrap_or_default();

    let asset = find_platform_asset(&latest.assets)?;

    let download_url = format!(
        "https://github.com/moekyawsoe/workspaces/releases/download/v{}/{}",
        version, asset.name
    );

    Ok((version, notes, download_url, asset.name.clone()))
}

fn find_platform_asset(
    assets: &[self_update::update::ReleaseAsset],
) -> Result<&self_update::update::ReleaseAsset, Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        assets
            .iter()
            .find(|a| a.name.ends_with(".deb"))
            .ok_or("No Linux release asset found".into())
    }
    #[cfg(target_os = "windows")]
    {
        assets
            .iter()
            .find(|a| a.name.ends_with(".zip"))
            .ok_or("No Windows release asset found".into())
    }
    #[cfg(target_os = "macos")]
    {
        assets
            .iter()
            .find(|a| a.name.ends_with(".dmg"))
            .ok_or("No macOS release asset found".into())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Err("Unsupported platform for auto-update".into())
    }
}

fn is_newer(latest: &str, current: &str) -> bool {
    let latest = latest.trim_start_matches('v');
    let current = current.trim_start_matches('v');

    let latest_parts: Vec<u64> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let current_parts: Vec<u64> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    for (l, c) in latest_parts.iter().zip(current_parts.iter()) {
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    latest_parts.len() > current_parts.len()
}

fn download_with_progress(
    url: &str,
    file_name: &str,
    status: &Arc<Mutex<UpdateStatus>>,
    ctx: &egui::Context,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let response = ureq::get(url)
        .set("User-Agent", "workspace-manager")
        .set("Accept", "application/octet-stream")
        .call()?;

    let content_type = response.header("Content-Type").unwrap_or("");
    if content_type.starts_with("text/html") || content_type.starts_with("text/plain") {
        return Err(format!("Download returned HTML/plain content instead of binary (Content-Type: {}). The release asset may not exist yet.", content_type).into());
    }

    let total_size: u64 = response
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let temp_dir = std::env::temp_dir();
    let download_path = temp_dir.join(file_name);

    let mut file = std::fs::File::create(&download_path)?;
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 8192];
    let mut downloaded: u64 = 0;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
        downloaded += n as u64;

        if total_size > 0 {
            let progress = (downloaded as f32 / total_size as f32).min(1.0);
            *status.lock().unwrap() = UpdateStatus::Downloading(progress);
            ctx.request_repaint();
        }
    }

    if total_size == 0 {
        *status.lock().unwrap() = UpdateStatus::Downloading(1.0);
        ctx.request_repaint();
    }

    if downloaded < 100 {
        let content = std::fs::read_to_string(&download_path).unwrap_or_default();
        if content.contains("<!DOCTYPE html>") || content.contains("<html>") {
            let _ = std::fs::remove_file(&download_path);
            return Err("Downloaded file appears to be an HTML page, not a binary. Check that the release tag and asset exist.".into());
        }
    }

    Ok(download_path)
}

fn install_binary(download_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        install_linux(download_path)?;
        restart_app();
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        install_windows(download_path)
    }
    #[cfg(target_os = "macos")]
    {
        install_macos(download_path)?;
        restart_app();
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn install_linux(download_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let magic = std::fs::read(download_path)?;
    if !magic.starts_with(b"!<arch>\n") {
        let preview = String::from_utf8_lossy(&magic[..magic.len().min(200)]);
        return Err(format!(
            "Downloaded file is not a valid .deb archive. First bytes: {}",
            preview
        )
        .into());
    }

    let extract_dir = std::env::temp_dir().join("workspace-manager-update");
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir)?;

    let output = std::process::Command::new("dpkg-deb")
        .arg("-x")
        .arg(download_path)
        .arg(&extract_dir)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to extract .deb: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let new_binary = extract_dir.join("usr/bin/workspace-manager");
    if !new_binary.exists() {
        return Err("Binary not found in extracted .deb".into());
    }

    replace_binary(&new_binary)?;

    let _ = std::fs::remove_dir_all(&extract_dir);
    let _ = std::fs::remove_file(download_path);

    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows(download_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(download_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let temp_dir = std::env::temp_dir().join("workspace-manager-update");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = temp_dir.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    let new_exe = temp_dir.join("workspace-manager.exe");
    if !new_exe.exists() {
        return Err("workspace-manager.exe not found in archive".into());
    }

    let current_exe = std::env::current_exe()?;
    let batch_path = std::env::temp_dir().join("workspace-manager-updater.bat");
    let batch_content = format!(
        "@echo off\r\n\
         timeout /t 2 /nobreak > nul\r\n\
         move /y \"{}\" \"{}\"\r\n\
         start \"\" \"{}\"\r\n\
         del \"%~f0\"\r\n",
        new_exe.display(),
        current_exe.display(),
        current_exe.display(),
    );
    std::fs::write(&batch_path, batch_content)?;

    std::process::Command::new("cmd")
        .arg("/c")
        .arg("start")
        .arg("")
        .arg(&batch_path)
        .spawn()?;

    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_file(download_path);

    std::process::exit(0);
}

#[cfg(target_os = "macos")]
fn install_macos(download_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("hdiutil")
        .arg("attach")
        .arg("-nobrowse")
        .arg("-readonly")
        .arg(download_path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to mount DMG: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mount_point = stdout
        .lines()
        .find(|line| line.contains("/Volumes/"))
        .and_then(|line| line.split_whitespace().last())
        .ok_or("Could not find DMG mount point")?;

    let app_source = std::path::Path::new(mount_point).join("Workspace Manager.app");
    if !app_source.exists() {
        let _ = std::process::Command::new("hdiutil")
            .arg("detach")
            .arg(mount_point)
            .output();
        return Err("Workspace Manager.app not found in DMG".into());
    }

    let current_exe = std::env::current_exe()?;
    let app_dest = current_exe
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or("Could not determine .app bundle location")?
        .to_path_buf();

    if !app_dest.extension().map_or(false, |ext| ext == "app") {
        let _ = std::process::Command::new("hdiutil")
            .arg("detach")
            .arg(mount_point)
            .output();
        return Err("App is not running from an .app bundle".into());
    }

    let _ = std::fs::remove_dir_all(&app_dest);

    let output = std::process::Command::new("cp")
        .arg("-R")
        .arg(&app_source)
        .arg(&app_dest)
        .output()?;

    if !output.status.success() {
        let _ = std::process::Command::new("hdiutil")
            .arg("detach")
            .arg(mount_point)
            .output();
        return Err(format!(
            "Failed to copy .app: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let _ = std::process::Command::new("hdiutil")
        .arg("detach")
        .arg(mount_point)
        .output();

    let _ = std::fs::remove_file(download_path);

    Ok(())
}

fn replace_binary(new_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    std::fs::copy(new_path, &current_exe)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&current_exe)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&current_exe, perms)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn restart_app() {
    let current_exe = std::env::current_exe().unwrap();
    let _ = std::process::Command::new(&current_exe)
        .args(std::env::args().skip(1))
        .spawn();
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
fn restart_app() {
    let current_exe = std::env::current_exe().unwrap();

    if let Some(app_path) = current_exe
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .filter(|p| p.extension().map_or(false, |ext| ext == "app"))
    {
        let _ = std::process::Command::new("open").arg(app_path).spawn();
    } else {
        let _ = std::process::Command::new(&current_exe).spawn();
    }
    std::process::exit(0);
}
