"""Generate and DSA-sign the WinSparkle appcast for a goresave release.

The appcast is uploaded as a GitHub release asset; the app polls
releases/latest/download/appcast-windows.xml. The enclosure URL points at
the versioned installer asset of the same release.

Usage:
    python scripts/appcast.py --version 0.1.1 \
        --installer dist/GoresaveSetup-0.1.1.exe \
        --notes dist/RELEASE_NOTES.md \
        --output dist/appcast-windows.xml

Environment:
    WINSPARKLE_DSA_PRIV_KEY_B64   base64-encoded DSA private key PEM.
                                  Required: WinSparkle rejects unsigned
                                  updates once a public key is embedded.
"""

from __future__ import annotations

import argparse
import base64
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from email.utils import format_datetime
from pathlib import Path

REPO_DOWNLOAD_BASE = "https://github.com/dh0er/goresave/releases/download"


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
        digest = subprocess.run(
            ["openssl", "dgst", "-sha1", "-binary"],
            input=installer.read_bytes(),
            capture_output=True,
            check=True,
        )
        result = subprocess.run(
            ["openssl", "dgst", "-sha1", "-sign", str(key_path)],
            input=digest.stdout,
            capture_output=True,
            check=True,
        )
    return base64.b64encode(result.stdout).decode()


def notes_to_html(notes_path: Path | None) -> str:
    if notes_path is None or not notes_path.exists():
        return ""
    lines = notes_path.read_text(encoding="utf-8").strip().split("\n")
    return "<br/>".join(line.strip() for line in lines if line.strip())


def to_cdata(text: str) -> str:
    return "<![CDATA[" + text.replace("]]>", "]]]]><![CDATA[>") + "]]>"


def build_appcast(*, version: str, installer: Path, notes_html: str,
                  signature: str) -> str:
    rss = ET.Element("rss", {
        "version": "2.0",
        "xmlns:sparkle": "http://www.andymatuschak.org/xml-namespaces/sparkle",
        "xmlns:dc": "http://purl.org/dc/elements/1.1/",
    })
    channel = ET.SubElement(rss, "channel")
    ET.SubElement(channel, "title").text = "goresave"

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
        "url": f"{REPO_DOWNLOAD_BASE}/v{version}/{installer.name}",
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
    parser.add_argument("--version", required=True)
    parser.add_argument("--installer", required=True, type=Path)
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    if not args.installer.exists():
        sys.exit(f"installer not found: {args.installer}")

    signature = sign_dsa(args.installer)
    xml = build_appcast(
        version=args.version,
        installer=args.installer,
        notes_html=notes_to_html(args.notes),
        signature=signature,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(xml, encoding="utf-8")
    print(f"appcast written: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
