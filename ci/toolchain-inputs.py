#!/usr/bin/env python3
"""Hash the toolchain's Dockerfile instructions and transitive COPY inputs, not the app."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shlex

ROOT = Path(__file__).resolve().parents[1]
VARIABLE = re.compile(r"\$(?:\{([A-Za-z_]\w*)[^}]*\}|([A-Za-z_]\w*))")


def instructions(text):
    lines = [line for line in text.splitlines() if line.strip() and not line.lstrip().startswith("#")]
    return re.sub(r"\\\n", " ", "\n".join(lines)).splitlines()


def toolchain_inputs(root):
    text = (root / "Dockerfile").read_text()
    sections = re.split(r"(?im)(?=^FROM\s)", text)
    global_args = dict(re.findall(r"^ARG (\w+)=(.*)$", "\n".join(instructions(sections[0])), re.M))
    stages = {}
    for section in sections[1:]:
        recipe = instructions(section)
        words = shlex.split(recipe[0])
        if len(words) >= 4 and words[-2].upper() == "AS":
            stages[words[-1]] = recipe

    selected, sources = {}, set()

    def visit(name):
        if name in selected:
            return
        recipe = stages[name]
        selected[name] = recipe
        base = next(word for word in shlex.split(recipe[0])[1:] if not word.startswith("--"))
        if base in stages:
            visit(base)
        for line in recipe[1:]:
            command, args = line.split(maxsplit=1)
            if command.upper() == "RUN" and "--mount=" in args:
                raise ValueError("Teach toolchain-inputs.py about RUN mounts before using them in the toolchain")
            if command.upper() not in {"COPY", "ADD"}:
                continue
            words = shlex.split(args)
            dependency = next((word.split("=", 1)[1] for word in words if word.startswith("--from=")), None)
            if dependency is not None:
                if dependency not in stages:
                    raise ValueError(f"Toolchain COPY must use a named stage: {dependency}")
                visit(dependency)
            else:
                paths = [word for word in words if not word.startswith("--")][:-1]
                for source in paths:
                    if any(char in source for char in "$*?[<") or "://" in source:
                        raise ValueError(f"Toolchain COPY needs a literal local path: {source}")
                    path = root / source
                    if not path.exists():
                        raise ValueError(f"Missing toolchain input: {source}")
                    files = sorted(path.rglob("*")) if path.is_dir() else [path]
                    sources.update(file.relative_to(root).as_posix() for file in files if file.is_file())

    visit("toolchain")
    referenced = set()
    pending = "\n".join(line for recipe in selected.values() for line in recipe)
    while pending:
        names = {a or b for a, b in VARIABLE.findall(pending)} - referenced
        referenced.update(names)
        pending = "\n".join(global_args[name] for name in names if name in global_args)
    # Only the registry location changes; the repository paths and digest pins stay in FROM.
    arguments = {name: global_args[name] for name in sorted(referenced & global_args.keys()) if name != "BASE_REGISTRY"}
    syntax = re.match(r"#\s*syntax\s*=\s*(.*)", text)
    recipe = {"syntax": syntax[1] if syntax else None, "args": arguments, "stages": selected}
    digest = hashlib.sha256(json.dumps(recipe, sort_keys=True).encode())
    for source in sorted(sources):
        path = root / source
        content = path.read_bytes()
        header = json.dumps([source, bool(path.stat().st_mode & 0o111), len(content)])
        digest.update(header.encode() + b"\0" + content)
    return digest.hexdigest(), sorted(sources)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--files", action="store_true", help="list copied inputs for the producer's push paths")
    args = parser.parse_args()
    digest, files = toolchain_inputs(ROOT)
    print("\n".join(["Dockerfile", *files]) if args.files else digest)
