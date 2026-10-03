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

fn installed_target(o: &Options) -> Option<PathBuf> {
    if o.kind.as_deref() == Some("portable") {
        return None;
    }
    if let Some(path) = &o.target {
        let root = if path.is_file() {
            path.parent()?.to_owned()
        } else {
            path.clone()
        };
        return root.join("unins000.exe").is_file().then_some(root);
    }
    #[cfg(windows)]
    for key in ["ProgramFiles", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(key) {
            for suffix in ["goresave", "Programs/goresave"] {
                let path = PathBuf::from(&root).join(suffix);
                if path.join("unins000.exe").is_file() {
                    return Some(path);
                }
            }
        }
    }
    None
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
    let page = format!(
        "https://github.com/dh0er/gore/releases/tag/gore-save-editor-v{}",
        release.version
    );
    let installed = installed_target(o);
    if installed.is_none()
        && (o.kind.as_deref() == Some("installed")
            || (o.target.is_some() && o.kind.as_deref() != Some("portable")))
    {
        bail!(
            "installed target must contain the Editor's unins000.exe; use --kind portable for a portable copy"
        );
    }
    let current = include_str!("../../../../../apps/save-editor/pubspec.yaml")
        .lines()
        .find_map(|s| s.strip_prefix("version:"))
        .map(str::trim)
        .unwrap_or("unknown");
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
    let mut result = json!({"updateAvailable":available,"product":"save-editor","bundledEditorVersion":current,"latestVersion":release.version,"release":page,"feed":FEED,"installer":release.url,"installedTarget":installed,"mode":if installed.is_some(){"installed"}else{"portable"},"dryRun":o.dry_run});
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
        let installed = installed.unwrap().canonicalize()?;
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
                let result = apply_release("install", &options, release(&version)).unwrap();
                assert_eq!(result["updateAvailable"], false);
                assert_eq!(result["action"], "up-to-date");
                assert_eq!(result["mode"], "installed");
                assert_eq!(fs::read(&uninstaller).unwrap(), b"existing installation");
                assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
            }
        }
        let simulated = Options {
            dry_run: true,
            ..options
        };
        let newer = apply_release("install", &simulated, release("9999.0.0")).unwrap();
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
    fn native_verification_matches_openssl_and_rejects_modified_installer() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/winsparkle-signature.json"
        ))
        .unwrap();
        let key = fixture["publicKey"].as_str().unwrap();
        let signature = fixture["signature"].as_str().unwrap();
        verify_dsa(b"abc", key, signature).unwrap();
        assert!(verify_dsa(b"abd", key, signature).is_err());
        assert!(
            verify_dsa(
                b"abc",
                include_str!("../../../../../apps/save-editor/dsa_pub.pem"),
                signature
            )
            .is_err()
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
