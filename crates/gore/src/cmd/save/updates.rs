//! The Save Editor's fixed appcast and WinSparkle signature contract.
use super::*;
use base64::Engine;
use std::process::Command;

const FEED: &str =
    "https://github.com/dh0er/gore/releases/download/gore-save-editor-appcast/appcast-windows.xml";
const RELEASES: &str = "https://github.com/dh0er/gore/releases/download/";

struct Release {
    version: String,
    url: String,
    length: u64,
    signature: String,
}
fn attribute(tag: &str, key: &str) -> Result<String> {
    let needle = format!("{key}=\"");
    let value = tag
        .split_once(&needle)
        .and_then(|(_, s)| s.split_once('"'))
        .map(|(v, _)| v)
        .with_context(|| format!("appcast enclosure has no {key}"))?;
    if value.contains('&') {
        bail!("unexpected escaped appcast attribute");
    }
    Ok(value.into())
}
fn parse(xml: &str) -> Result<Release> {
    let version = xml
        .split_once("<sparkle:version>")
        .and_then(|(_, s)| s.split_once("</sparkle:version>"))
        .map(|(v, _)| v.trim())
        .context("appcast has no version")?;
    let parts = version.split('.').collect::<Vec<_>>();
    if !(3..=4).contains(&parts.len())
        || parts
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()))
    {
        bail!("invalid appcast version");
    }
    let tag = xml
        .split_once("<enclosure ")
        .and_then(|(_, s)| s.split_once('>'))
        .map(|(v, _)| v)
        .context("appcast has no enclosure")?;
    let url = attribute(tag, "url")?;
    let expected =
        format!("{RELEASES}gore-save-editor-v{version}/gore-save-editor-{version}-setup.exe");
    if url != expected {
        bail!("appcast installer URL does not match the advertised Save Editor release");
    }
    let signature = attribute(tag, "sparkle:dsaSignature")?;
    if base64::engine::general_purpose::STANDARD
        .decode(&signature)?
        .is_empty()
    {
        bail!("empty installer signature");
    }
    let length = attribute(tag, "length")?.parse::<u64>()?;
    if length == 0 || length > 1024 * 1024 * 1024 {
        bail!("invalid installer length");
    }
    Ok(Release {
        version: version.into(),
        url,
        length,
        signature,
    })
}

fn curl(url: &str, out: Option<&Path>, limit: u64) -> Result<Vec<u8>> {
    let mut command = Command::new("curl");
    command.args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--max-time",
        "120",
        "--max-filesize",
        &limit.to_string(),
    ]);
    if let Some(path) = out {
        command.arg("--output").arg(path);
    }
    let output = command
        .arg(url)
        .output()
        .context("update check/download requires curl")?;
    if !output.status.success() {
        bail!(
            "update request failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout)
}

// DSA SubjectPublicKeyInfo and DER signatures used by WinSparkle. Parsing is
// bounded and rejects trailing data; the trusted key ships with the Editor.
fn der<'a>(bytes: &mut &'a [u8], tag: u8) -> Result<&'a [u8]> {
    if bytes.len() < 2 || bytes[0] != tag {
        bail!("invalid DSA DER tag");
    }
    let first = bytes[1];
    let (length, header) = if first < 128 {
        (first as usize, 2)
    } else {
        let count = (first & 127) as usize;
        if count == 0 || count > 4 || bytes.len() < 2 + count || bytes[2] == 0 {
            bail!("invalid DSA DER length");
        }
        let length = bytes[2..2 + count]
            .iter()
            .fold(0usize, |n, b| (n << 8) | *b as usize);
        if length < 128 {
            bail!("noncanonical DSA DER length");
        }
        (length, 2 + count)
    };
    if length > 8192 || bytes.len() < header + length {
        bail!("truncated or oversized DSA DER value");
    }
    let value = &bytes[header..header + length];
    *bytes = &bytes[header + length..];
    Ok(value)
}
fn integer(bytes: &mut &[u8]) -> Result<num_bigint::BigUint> {
    let value = der(bytes, 2)?;
    if value.is_empty()
        || value[0] & 128 != 0
        || (value.len() > 1 && value[0] == 0 && value[1] & 128 == 0)
    {
        bail!("invalid positive DSA integer");
    }
    Ok(num_bigint::BigUint::from_bytes_be(value))
}
fn verify_dsa(data: &[u8], pem: &str, signature: &str) -> Result<()> {
    use num_bigint::BigUint;
    use sha1::{Digest, Sha1};
    let pem = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();
    let key = base64::engine::general_purpose::STANDARD.decode(pem)?;
    let mut outer = key.as_slice();
    let mut spki = der(&mut outer, 0x30)?;
    if !outer.is_empty() {
        bail!("trailing public key bytes");
    }
    let mut algorithm = der(&mut spki, 0x30)?;
    if der(&mut algorithm, 6)? != [0x2a, 0x86, 0x48, 0xce, 0x38, 0x04, 0x01] {
        bail!("public key is not DSA");
    }
    let mut parameters = der(&mut algorithm, 0x30)?;
    let p = integer(&mut parameters)?;
    let q = integer(&mut parameters)?;
    let g = integer(&mut parameters)?;
    let bits = der(&mut spki, 3)?;
    if bits.first() != Some(&0) {
        bail!("invalid public key bit string");
    }
    let mut bits = &bits[1..];
    let y = integer(&mut bits)?;
    if !algorithm.is_empty() || !parameters.is_empty() || !spki.is_empty() || !bits.is_empty() {
        bail!("trailing DSA key data");
    }
    let encoded = base64::engine::general_purpose::STANDARD.decode(signature)?;
    let mut encoded = encoded.as_slice();
    let mut seq = der(&mut encoded, 0x30)?;
    let r = integer(&mut seq)?;
    let s = integer(&mut seq)?;
    let one = BigUint::from(1u8);
    let zero = BigUint::from(0u8);
    if !encoded.is_empty()
        || !seq.is_empty()
        || p.bits() > 4096
        || q.bits() < 160
        || q.bits() > 256
        || g <= one
        || g >= p
        || y <= one
        || y >= p
        || r == zero
        || r >= q
        || s == zero
        || s >= q
    {
        bail!("invalid DSA signature or key parameters");
    }
    // scripts/appcast.py signs SHA1(installer) with openssl dgst -sha1;
    // WinSparkle therefore verifies SHA1(SHA1(installer)).
    let digest = Sha1::digest(Sha1::digest(data));
    let z = BigUint::from_bytes_be(&digest);
    let w = s.modpow(&(&q - BigUint::from(2u8)), &q);
    let u1 = (&z * &w) % &q;
    let u2 = (&r * &w) % &q;
    let v = (g.modpow(&u1, &p) * y.modpow(&u2, &p) % &p) % &q;
    if v != r {
        bail!("installer signature does not match the public key embedded in Save Editor");
    }
    Ok(())
}
fn verify(installer: &Path, release: &Release) -> Result<()> {
    if fs::metadata(installer)?.len() != release.length {
        bail!("downloaded installer size differs from the signed appcast");
    }
    verify_dsa(
        &fs::read(installer)?,
        include_str!("../../../../../apps/save-editor/dsa_pub.pem"),
        &release.signature,
    )
}

struct Installation {
    root: PathBuf,
    version: String,
}

#[cfg(windows)]
fn editor_version(executable: &Path) -> Result<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    };
    let name: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
    let mut ignored = 0;
    // SAFETY: name is a terminated UTF-16 path; ignored is writable.
    let size = unsafe { GetFileVersionInfoSizeW(name.as_ptr(), &mut ignored) };
    if size == 0 || size > 1024 * 1024 {
        bail!("goresave.exe has no readable version metadata");
    }
    let mut bytes = vec![0u8; size as usize];
    // SAFETY: the allocation holds exactly the size returned by the version API.
    if unsafe { GetFileVersionInfoW(name.as_ptr(), 0, size, bytes.as_mut_ptr().cast()) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    fn query<'a>(bytes: &'a [u8], key: &str, unit: usize) -> Result<&'a [u8]> {
        let key: Vec<u16> = key.encode_utf16().chain([0]).collect();
        let mut pointer = std::ptr::null_mut();
        let mut length = 0;
        // SAFETY: bytes is a complete version block; key and output pointers are valid.
        if unsafe {
            VerQueryValueW(
                bytes.as_ptr().cast(),
                key.as_ptr(),
                &mut pointer,
                &mut length,
            )
        } == 0
        {
            bail!("goresave.exe is missing required version metadata");
        }
        let start = (pointer as usize)
            .checked_sub(bytes.as_ptr() as usize)
            .context("invalid version metadata pointer")?;
        let end = (length as usize)
            .checked_mul(unit)
            .and_then(|length| start.checked_add(length))
            .context("invalid version metadata length")?;
        bytes
            .get(start..end)
            .context("version metadata exceeds its buffer")
    }
    fn text(bytes: &[u8], key: &str) -> Result<String> {
        let bytes = query(bytes, key, 2)?;
        let units: Vec<_> = bytes
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        Ok(String::from_utf16(&units)?)
    }
    let translations = query(&bytes, "\\VarFileInfo\\Translation", 1)?;
    if translations.is_empty() || translations.len() % 4 != 0 {
        bail!("goresave.exe has invalid version translations");
    }
    let identified = translations.chunks_exact(4).any(|translation| {
        let language = u16::from_le_bytes([translation[0], translation[1]]);
        let codepage = u16::from_le_bytes([translation[2], translation[3]]);
        let prefix = format!("\\StringFileInfo\\{language:04x}{codepage:04x}");
        text(&bytes, &format!("{prefix}\\ProductName")).is_ok_and(|name| name == "GORE Save Editor")
            && text(&bytes, &format!("{prefix}\\OriginalFilename"))
                .is_ok_and(|name| name.eq_ignore_ascii_case("goresave.exe"))
    });
    if !identified {
        bail!("goresave.exe version metadata does not identify GORE Save Editor");
    }
    let fixed = query(&bytes, "\\", 1)?;
    if fixed.len() < std::mem::size_of::<VS_FIXEDFILEINFO>() {
        bail!("goresave.exe has truncated version metadata");
    }
    // SAFETY: the bounds above cover this plain Win32 struct; alignment is not assumed.
    let fixed = unsafe { std::ptr::read_unaligned(fixed.as_ptr().cast::<VS_FIXEDFILEINFO>()) };
    if fixed.dwSignature != 0xfeef04bd {
        bail!("goresave.exe has invalid fixed version metadata");
    }
    Ok(format!(
        "{}.{}.{}.{}",
        fixed.dwProductVersionMS >> 16,
        fixed.dwProductVersionMS & 0xffff,
        fixed.dwProductVersionLS >> 16,
        fixed.dwProductVersionLS & 0xffff
    ))
}

#[cfg(not(windows))]
fn editor_version(_executable: &Path) -> Result<String> {
    bail!("installed Save Editor version detection is supported on Windows")
}

fn installed_target_with(
    o: &Options,
    read_version: impl Fn(&Path) -> Result<String>,
) -> Result<Option<Installation>> {
    if o.kind.as_deref() == Some("portable") {
        return Ok(None);
    }
    let inspect = |root: PathBuf| -> Result<Installation> {
        let executable = root.join("goresave.exe");
        if !root.join("unins000.exe").is_file() || !executable.is_file() {
            bail!("installed target must contain goresave.exe and the Editor's unins000.exe; use --kind portable for a portable copy");
        }
        Ok(Installation {
            version: read_version(&executable)?,
            root,
        })
    };
    if let Some(path) = &o.target {
        let root = if path.is_file() {
            path.parent()
                .context("installed target has no parent")?
                .to_owned()
        } else {
            path.clone()
        };
        return inspect(root).map(Some);
    }
    #[cfg(windows)]
    for key in ["ProgramFiles", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(key) {
            for suffix in ["goresave", "Programs/goresave"] {
                let path = PathBuf::from(&root).join(suffix);
                if let Ok(installed) = inspect(path) {
                    return Ok(Some(installed));
                }
            }
        }
    }
    Ok(None)
}

pub(super) fn run(verb: &str, o: &Options) -> Result<Value> {
    if o.product != "save-editor" {
        bail!("--product must be save-editor");
    }
    if o.kind
        .as_deref()
        .is_some_and(|kind| !matches!(kind, "installed" | "portable"))
    {
        bail!("--kind must be installed or portable");
    }
    let xml = String::from_utf8(curl(FEED, None, 1024 * 1024)?)?;
    let release = parse(&xml)?;
    apply_release(verb, o, release)
}

fn apply_release(verb: &str, o: &Options, release: Release) -> Result<Value> {
    apply_release_with(verb, o, release, editor_version)
}

fn apply_release_with(
    verb: &str,
    o: &Options,
    release: Release,
    read_version: impl Fn(&Path) -> Result<String>,
) -> Result<Value> {
    let page = format!(
        "https://github.com/dh0er/gore/releases/tag/gore-save-editor-v{}",
        release.version
    );
    let installed = installed_target_with(o, read_version)?;
    if installed.is_none()
        && (o.kind.as_deref() == Some("installed")
            || (o.target.is_some() && o.kind.as_deref() != Some("portable")))
    {
        bail!(
            "no verified Save Editor installation found; use --target to select one or --kind portable for a portable copy"
        );
    }
    let bundled = include_str!("../../../../../apps/save-editor/pubspec.yaml")
        .lines()
        .find_map(|s| s.strip_prefix("version:"))
        .map(str::trim)
        .unwrap_or("unknown");
    let current = installed
        .as_ref()
        .map(|installed| installed.version.as_str())
        .unwrap_or(bundled);
    let numbers = |v: &str| {
        let mut parts = [0u32; 4];
        for (part, value) in parts
            .iter_mut()
            .zip(v.split('+').next().unwrap_or(v).split('.'))
        {
            *part = value.parse().unwrap_or(0);
        }
        parts
    };
    let available = numbers(&release.version) > numbers(current);
    let mut result = json!({"updateAvailable":available,"product":"save-editor","bundledEditorVersion":bundled,"installedEditorVersion":installed.as_ref().map(|installed| &installed.version),"currentEditorVersion":current,"latestVersion":release.version,"release":page,"feed":FEED,"installer":release.url,"installedTarget":installed.as_ref().map(|installed| &installed.root),"mode":if installed.is_some(){"installed"}else{"portable"},"dryRun":o.dry_run});
    if verb == "open-release" || (verb == "install" && installed.is_none()) {
        // The portable Editor opens this same download page; it never replaces
        // an unpacked directory in place or launches an installer implicitly.
        if !o.dry_run {
            display::open(Path::new(&page))?;
        }
        result["action"] = json!("download-page");
        return Ok(result);
    }
    if verb == "install" && !available {
        result["action"] = json!("up-to-date");
        return Ok(result);
    }
    if verb != "install" || o.dry_run {
        return Ok(result);
    }
    #[cfg(not(windows))]
    bail!("installed Save Editor updates are supported on Windows");
    #[cfg(windows)]
    {
        let installed = installed.unwrap().root.canonicalize()?;
        let temporary = tempfile::tempdir()?;
        let installer = temporary.path().join("setup.exe");
        curl(&release.url, Some(&installer), release.length)?;
        verify(&installer, &release)?;
        let status = Command::new(&installer)
            .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
            .arg(format!("/DIR={}", installed.display()))
            .status()?;
        if !status.success() {
            bail!("Save Editor installer failed: {status}");
        }
        result["action"] = json!("installed");
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_updates_skip_older_and_equal_feeds_before_download_or_execution() {
        let temp = tempfile::tempdir().unwrap();
        let uninstaller = temp.path().join("unins000.exe");
        fs::write(&uninstaller, b"existing installation").unwrap();
        fs::write(temp.path().join("goresave.exe"), b"test Editor executable").unwrap();
        let options = Options {
            target: Some(temp.path().to_owned()),
            kind: Some("installed".into()),
            ..Options::default()
        };
        let current = include_str!("../../../../../apps/save-editor/pubspec.yaml")
            .lines()
            .find_map(|s| s.strip_prefix("version:"))
            .unwrap()
            .trim();
        let current = current.split('+').next().unwrap();
        let release = |version: &str| Release {
            version: version.into(),
            // A download attempt would fail: old/equal feeds must return before
            // inspecting either the installer URL or its signature.
            url: "must-not-be-downloaded".into(),
            length: 1,
            signature: "not-a-signature".into(),
        };
        for version in [
            "0.0.0".to_owned(),
            current.to_owned(),
            format!("{current}.0"),
        ] {
            for dry_run in [false, true] {
                let options = Options {
                    dry_run,
                    ..options.clone()
                };
                let result = apply_release_with("install", &options, release(&version), |_| {
                    Ok(current.to_owned())
                })
                .unwrap();
                assert_eq!(result["updateAvailable"], false);
                assert_eq!(result["action"], "up-to-date");
                assert_eq!(result["mode"], "installed");
                assert_eq!(fs::read(&uninstaller).unwrap(), b"existing installation");
                assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
            }
        }
        let simulated = Options {
            dry_run: true,
            ..options
        };
        let newer = apply_release_with("install", &simulated, release("9999.0.0"), |_| {
            Ok(current.to_owned())
        })
        .unwrap();
        assert_eq!(newer["updateAvailable"], true);
        assert!(newer.get("action").is_none());
        let portable = Options {
            kind: Some("portable".into()),
            ..simulated
        };
        let manual = apply_release("install", &portable, release(current)).unwrap();
        assert_eq!(manual["action"], "download-page");
        assert_eq!(manual["updateAvailable"], false);
    }

    #[test]
    fn installed_update_targets_require_the_editor_and_readable_product_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let uninstaller = temp.path().join("unins000.exe");
        fs::write(&uninstaller, b"another Inno application").unwrap();
        let options = Options {
            target: Some(temp.path().to_owned()),
            kind: Some("installed".into()),
            ..Default::default()
        };
        assert!(installed_target_with(&options, |_| panic!(
            "missing Editor must be rejected first"
        ))
        .is_err());
        let executable = temp.path().join("goresave.exe");
        fs::write(&executable, b"unrelated file with an Editor filename").unwrap();
        for target in [temp.path().to_owned(), executable.clone()] {
            let options = Options {
                target: Some(target),
                ..options.clone()
            };
            let error = apply_release(
                "install",
                &options,
                Release {
                    version: "9999.0.0".into(),
                    url: "must-not-be-downloaded".into(),
                    length: 1,
                    signature: "bad".into(),
                },
            )
            .unwrap_err();
            assert!(error.to_string().contains("version"), "{error}");
            assert_eq!(fs::read(&uninstaller).unwrap(), b"another Inno application");
            assert_eq!(
                fs::read(&executable).unwrap(),
                b"unrelated file with an Editor filename"
            );
        }
        let portable = Options {
            kind: Some("portable".into()),
            ..options
        };
        assert!(installed_target_with(&portable, |_| panic!(
            "portable copies have no installed version"
        ))
        .unwrap()
        .is_none());
    }

    #[test]
    fn installed_feed_comparison_uses_the_selected_executables_version() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("unins000.exe"), b"installation").unwrap();
        fs::write(temp.path().join("goresave.exe"), b"test Editor executable").unwrap();
        let options = Options {
            target: Some(temp.path().to_owned()),
            kind: Some("installed".into()),
            dry_run: true,
            ..Default::default()
        };
        let bundled = include_str!("../../../../../apps/save-editor/pubspec.yaml")
            .lines()
            .find_map(|s| s.strip_prefix("version:"))
            .unwrap()
            .trim();
        let release = || Release {
            version: bundled.split('+').next().unwrap().into(),
            url: "must-not-be-downloaded".into(),
            length: 1,
            signature: "bad".into(),
        };
        for (installed, available) in [("0.0.0.0", true), ("9999.0.0.0", false)] {
            let result =
                apply_release_with("install", &options, release(), |_| Ok(installed.into()))
                    .unwrap();
            assert_eq!(result["updateAvailable"], available);
            assert_eq!(result["currentEditorVersion"], installed);
            assert_eq!(result["installedEditorVersion"], installed);
            assert_eq!(result["bundledEditorVersion"], bundled);
            assert_eq!(result["installedTarget"], json!(temp.path()));
            assert_eq!(result["mode"], "installed");
            if !available {
                assert_eq!(result["action"], "up-to-date");
            } else {
                assert!(result.get("action").is_none());
            }
        }
    }
    #[test]
    fn native_verification_matches_openssl_and_rejects_modified_installer() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/winsparkle-signature.json"
        ))
        .unwrap();
        let key = fixture["publicKey"].as_str().unwrap();
        let signature = fixture["signature"].as_str().unwrap();
        verify_dsa(b"abc", key, signature).unwrap();
        assert!(verify_dsa(b"abd", key, signature).is_err());
        assert!(verify_dsa(
            b"abc",
            include_str!("../../../../../apps/save-editor/dsa_pub.pem"),
            signature
        )
        .is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_installed_targets_validate_real_editor_version_resources() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/save-editor-version.json"
        ))
        .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(fixture["peBase64"].as_str().unwrap())
            .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("goresave.exe");
        fs::write(&executable, &bytes).unwrap();
        fs::write(temp.path().join("unins000.exe"), b"existing installation").unwrap();
        let options = Options {
            target: Some(executable.clone()),
            kind: Some("installed".into()),
            dry_run: true,
            ..Default::default()
        };
        let installed = installed_target_with(&options, editor_version)
            .unwrap()
            .unwrap();
        assert_eq!(installed.version, "1.3.0.0");
        assert_eq!(installed.root, temp.path());
        let result = apply_release(
            "install",
            &options,
            Release {
                version: "1.5.0".into(),
                url: "must-not-be-downloaded".into(),
                length: 1,
                signature: "bad".into(),
            },
        )
        .unwrap();
        assert_eq!(result["updateAvailable"], true);
        assert_eq!(result["installedEditorVersion"], "1.3.0.0");
        assert_eq!(fs::read(&executable).unwrap(), bytes);

        for (original, unrelated) in [
            ("GORE Save Editor", "GORE Mod Manager"),
            ("goresave.exe", "otherapp.exe"),
        ] {
            let encode = |value: &str| {
                value
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<_>>()
            };
            let original = encode(original);
            let unrelated = encode(unrelated);
            assert_eq!(original.len(), unrelated.len());
            let mut changed = bytes.clone();
            let offset = changed
                .windows(original.len())
                .position(|window| window == original)
                .unwrap();
            changed[offset..offset + original.len()].copy_from_slice(&unrelated);
            fs::write(&executable, changed).unwrap();
            let error = installed_target_with(&options, editor_version)
                .err()
                .unwrap();
            assert!(
                error
                    .to_string()
                    .contains("does not identify GORE Save Editor"),
                "{error}"
            );
        }
        fs::write(&executable, &bytes[..32]).unwrap();
        assert!(installed_target_with(&options, editor_version).is_err());
        assert_eq!(
            fs::read(temp.path().join("unins000.exe")).unwrap(),
            b"existing installation"
        );
    }
    #[test]
    fn appcast_cannot_redirect_installation_to_another_product_or_unsigned_file() {
        let xml = "<sparkle:version>1.4.1</sparkle:version><enclosure url=\"https://github.com/dh0er/gore/releases/download/gore-save-editor-v1.4.1/gore-save-editor-1.4.1-setup.exe\" length=\"3\" sparkle:dsaSignature=\"YWJj\"/>";
        let release = parse(xml).unwrap();
        assert_eq!(release.version, "1.4.1");
        assert!(parse(&xml.replace("/gore-save-editor-v", "/gore-mod-studio-v")).is_err());
        assert!(parse(&xml.replace("YWJj", "")).is_err());
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("setup.exe");
        fs::write(&file, b"abc").unwrap();
        assert!(verify(&file, &release).is_err());
    }
}
