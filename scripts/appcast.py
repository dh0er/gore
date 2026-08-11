"""Generate and DSA-sign a WinSparkle appcast for a gore release.

Shared by every WinSparkle-updated app in the monorepo (gore-save, gore-mod).
The appcast is uploaded as a GitHub release asset; the app polls a stable
feed URL (gore-save uses releases/latest/download/, gore-mod uses its own
fixed-tag release). The enclosure URL points at the versioned installer asset
of the matching per-project release tag (e.g. gore-save-editor-v1.2.3).

Usage:
    python scripts/appcast.py --title gore-save-editor --version 0.1.1 \
        --installer dist/gore-save-editor-0.1.1-setup.exe \
        --public-key apps/save-editor/dsa_pub.pem \
        --notes dist/RELEASE_NOTES.md \
        --release-tag gore-save-editor-v0.1.1 \
        --output dist/appcast-windows.xml

Environment:
    WINSPARKLE_DSA_PRIV_KEY_B64   base64-encoded DSA private key PEM.
                                  Required: WinSparkle rejects unsigned
                                  updates once a public key is embedded.
                                  All three apps currently share one keypair,
                                  so the same secret signs every feed.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from email.utils import format_datetime
from pathlib import Path

REPO_DOWNLOAD_BASE = "https://github.com/dh0er/gore/releases/download"


def sign_dsa(installer: Path) -> str:
    """Return the base64 DSA signature of *installer*.

    WinSparkle's verifier expects the two-stage scheme of its bundled
    sign_update.bat: the file's raw SHA-1 digest is piped into a second
    SHA-1+DSA signing step (sign(SHA1(SHA1(file)))). Signing the file in
    one pass produces signatures the embedded public key rejects.
    """
    key_b64 = os.environ.get("WINSPARKLE_DSA_PRIV_KEY_B64")
    if not key_b64:
        sys.exit("WINSPARKLE_DSA_PRIV_KEY_B64 is not set; refusing to "
                 "produce an unsigned appcast the app would reject")
    key_pem = base64.b64decode(key_b64).decode()
    with tempfile.TemporaryDirectory() as tmp:
        key_path = Path(tmp) / "dsa_priv.pem"
        key_path.write_text(key_pem, encoding="utf-8")
        digest = _sha1_digest(installer.read_bytes())
        result = subprocess.run(
            ["openssl", "dgst", "-sha1", "-sign", str(key_path)],
            input=digest,
            capture_output=True,
            check=True,
        )
    return base64.b64encode(result.stdout).decode()


def _sha1_digest(payload: bytes) -> bytes:
    return subprocess.run(
        ["openssl", "dgst", "-sha1", "-binary"],
        input=payload,
        capture_output=True,
        check=True,
    ).stdout


def verify_dsa(installer: Path, signature: str, public_key: Path) -> None:
    """Fail unless *signature* is accepted by the key embedded in the app.

    This mirrors WinSparkle's two-stage verification: the DSA signature covers
    SHA1(SHA1(installer)). A stale or wrong CI secret must stop the release
    before an appcast that every installed client rejects can be uploaded.
    """
    if not public_key.is_file():
        sys.exit(f"WinSparkle public key not found: {public_key}")
    try:
        signature_bytes = base64.b64decode(signature, validate=True)
    except (ValueError, binascii.Error) as error:
        raise SystemExit(f"invalid base64 DSA signature: {error}") from error
    if not signature_bytes:
        sys.exit("empty DSA signature")

    digest = _sha1_digest(installer.read_bytes())
    with tempfile.TemporaryDirectory() as tmp:
        signature_path = Path(tmp) / "installer.dsa"
        signature_path.write_bytes(signature_bytes)
        verified = subprocess.run(
            [
                "openssl",
                "dgst",
                "-sha1",
                "-verify",
                str(public_key),
                "-signature",
                str(signature_path),
            ],
            input=digest,
            capture_output=True,
            check=False,
        )
    if verified.returncode != 0:
        sys.exit(
            "DSA signature does not match the WinSparkle public key "
            f"embedded by this release: {public_key}"
        )


def notes_to_html(notes_path: Path | None) -> str:
    if notes_path is None or not notes_path.exists():
        return ""
    lines = notes_path.read_text(encoding="utf-8").strip().split("\n")
    return "<br/>".join(line.strip() for line in lines if line.strip())


def to_cdata(text: str) -> str:
    return "<![CDATA[" + text.replace("]]>", "]]]]><![CDATA[>") + "]]>"


def build_appcast(*, title: str, version: str, installer: Path,
                  notes_html: str, signature: str, release_tag: str) -> str:
    rss = ET.Element("rss", {
        "version": "2.0",
        "xmlns:sparkle": "http://www.andymatuschak.org/xml-namespaces/sparkle",
        "xmlns:dc": "http://purl.org/dc/elements/1.1/",
    })
    channel = ET.SubElement(rss, "channel")
    ET.SubElement(channel, "title").text = title

    item = ET.SubElement(channel, "item")
    ET.SubElement(item, "title").text = f"Version {version}"
    ET.SubElement(item, "sparkle:version").text = version
    ET.SubElement(item, "sparkle:shortVersionString").text = version
    ET.SubElement(item, "pubDate").text = format_datetime(
        datetime.now(timezone.utc))

    placeholder = None
    if notes_html:
        placeholder = "__APPCAST_DESCRIPTION_HTML__"
        ET.SubElement(item, "description").text = placeholder

    ET.SubElement(item, "enclosure", {
        "url": f"{REPO_DOWNLOAD_BASE}/{release_tag}/{installer.name}",
        "length": str(installer.stat().st_size),
        "type": "application/octet-stream",
        "sparkle:dsaSignature": signature,
    })

    ET.indent(rss, space="  ")
    xml = ET.tostring(rss, encoding="unicode", xml_declaration=True)
    if placeholder:
        xml = xml.replace(
            f"<description>{placeholder}</description>",
            f"<description>{to_cdata(notes_html)}</description>",
            1,
        )
    return xml + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--title", default="gore-save-editor",
                        help="channel title (the product name)")
    parser.add_argument("--version", required=True)
    parser.add_argument("--installer", required=True, type=Path)
    parser.add_argument(
        "--public-key",
        required=True,
        type=Path,
        help="WinSparkle DSA public key embedded in this app",
    )
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--release-tag", required=True,
                        help="git tag the installer asset lives under, "
                        "e.g. gore-save-editor-v1.2.3")
    args = parser.parse_args()

    if not args.installer.exists():
        sys.exit(f"installer not found: {args.installer}")

    signature = sign_dsa(args.installer)
    verify_dsa(args.installer, signature, args.public_key)
    xml = build_appcast(
        title=args.title,
        version=args.version,
        installer=args.installer,
        notes_html=notes_to_html(args.notes),
        signature=signature,
        release_tag=args.release_tag,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(xml, encoding="utf-8")
    print(f"appcast written: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
