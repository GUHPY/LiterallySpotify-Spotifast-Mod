"""Sign and verify the release manifest consumed by fastframe-update."""
import argparse
import hashlib
import os
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)
from cryptography.hazmat.primitives import serialization


def sign(directory: Path, public_key: Path, private_hex: str) -> None:
    key = Ed25519PrivateKey.from_private_bytes(bytes.fromhex(private_hex.strip()))
    expected = bytes.fromhex(public_key.read_text(encoding="ascii").strip())
    actual = key.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    if actual != expected:
        raise ValueError("Signing key does not match the application's publisher key")
    files = sorted(p for p in directory.iterdir() if p.is_file() and p.suffix in {".exe", ".zip"})
    if not files:
        raise ValueError("No release binaries to sign")
    manifest = "".join(
        f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n"
        for path in files
    ).encode("utf-8")
    (directory / "checksums.txt").write_bytes(manifest)
    signature = key.sign(manifest)
    Ed25519PublicKey.from_public_bytes(expected).verify(signature, manifest)
    (directory / "checksums.txt.sig").write_bytes(signature)
    print(f"Signed and verified {len(files)} release assets")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--public-key", type=Path, default=Path("assets/update-public-key.hex"))
    parser.add_argument("--key-file", type=Path)
    args = parser.parse_args()
    secret = (
        args.key_file.read_text(encoding="ascii")
        if args.key_file
        else os.environ.get("LITERALLYSPOTIFY_UPDATE_SIGNING_KEY", "")
    )
    if not secret:
        parser.error("Set LITERALLYSPOTIFY_UPDATE_SIGNING_KEY or provide --key-file")
    sign(args.directory, args.public_key, secret)
