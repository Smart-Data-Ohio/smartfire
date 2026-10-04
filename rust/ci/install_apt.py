#!/usr/bin/env python3
"""Install only committed, checksum-verified Debian archives; never resolve online."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


def validate_lock(lock):
    names = set()
    for package in lock["packages"]:
        name = package["package"]
        if name in names or not re.fullmatch(r"[a-z0-9][a-z0-9+.-]+", name):
            raise ValueError(f"invalid or duplicate package: {name}")
        names.add(name)
        if not package["version"] or not re.fullmatch(r"[0-9a-f]{64}", package["sha256"]):
            raise ValueError(f"package lacks an immutable pin: {name}")
        if package["architecture"] not in ("all", lock["architecture"]):
            raise ValueError(f"wrong package architecture: {name}")
        if not package["url"].startswith(("https://deb.debian.org/debian/pool/", "https://deb.debian.org/debian-security/pool/")):
            raise ValueError(f"unexpected Debian archive URL: {name}")
    if not set(lock["requested"]) <= names:
        raise ValueError("a requested prerequisite is absent from the checksum lock")


def verify_deb(package, archive):
    with archive.open("rb") as source:
        actual = hashlib.file_digest(source, "sha256").hexdigest()
    if actual != package["sha256"]:
        raise ValueError(f"checksum mismatch for {package['package']}: {actual}")
    identity = subprocess.check_output([
        "dpkg-deb", "--show", "--showformat", "${Package}\t${Version}\t${Architecture}", str(archive),
    ], text=True).split("\t")
    if identity != [package["package"], package["version"], package["architecture"]]:
        raise ValueError(f"archive identity mismatch for {package['package']}: {identity}")


def validate_dependency_graph(packages, metadata):
    # Require a locked provider for every runtime dependency, even when the
    # toolchain base already happens to contain a package that could satisfy it.
    providers = {package["package"] for package in packages}
    for _, _, provided in metadata.values():
        providers.update(part.strip().split()[0].split(":")[0]
                         for part in provided.split(",") if part.strip())
    for name, (depends, predepends, _) in metadata.items():
        for dependency in (depends + "," + predepends).split(","):
            if not dependency.strip():
                continue
            alternatives = {part.strip().split()[0].split(":")[0] for part in dependency.split("|")}
            if not alternatives & providers:
                raise ValueError(f"unlocked runtime dependency of {name}: {dependency}")


def install(lock):
    validate_lock(lock)
    architecture = subprocess.check_output(["dpkg", "--print-architecture"], text=True).strip()
    if architecture != lock["architecture"]:
        raise ValueError(f"this prerequisite lock supports {lock['architecture']}, not {architecture}")
    with tempfile.TemporaryDirectory(prefix="ci-apt-") as scratch:
        archives = []
        for package in lock["packages"]:
            archive = Path(scratch) / (package["package"] + ".deb")
            subprocess.run(["curl", "--fail", "--silent", "--show-error", "--location",
                            "--proto", "=https", "--proto-redir", "=https", package["url"], "--output", str(archive)], check=True)
            verify_deb(package, archive)
            archives.append(str(archive))
        # All bytes and identities are verified before package scripts can execute.
        # No repository update, dependency download, or unpinned fallback is allowed.
        metadata = {
            package["package"]: subprocess.check_output([
                "dpkg-deb", "--show", "--showformat", "${Depends}\t${Pre-Depends}\t${Provides}", archive,
            ], text=True).split("\t")
            for package, archive in zip(lock["packages"], archives)
        }
        validate_dependency_graph(lock["packages"], metadata)
        subprocess.run(["dpkg", "--install", *archives], check=True)
        audit = subprocess.check_output(["dpkg", "--audit"], text=True)
        if audit.strip():
            raise ValueError(f"Debian package graph is incomplete: {audit}")
        for package in lock["packages"]:
            status = subprocess.check_output(["dpkg-query", "--show", "--showformat", "${db:Status-Status}\t${Version}", package["package"]], text=True)
            if status != "installed\t" + package["version"]:
                raise ValueError(f"installed version differs from lock: {package['package']}")
    print(f"APT prerequisite receipt: {len(lock['packages'])} checksum-pinned packages installed")


if __name__ == "__main__":
    install(json.loads(Path(sys.argv[1]).read_text()))
