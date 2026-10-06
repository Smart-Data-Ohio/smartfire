"""Every file a Dockerfile copies from its build context survives that context's .dockerignore.

The image builds from the repository root, so the root .dockerignore must keep out everything
but the image's inputs, and a pattern that hides one of them otherwise fails only when the image
is built on main. This reads the patterns the way BuildKit does (moby/patternmatcher: the last
matching pattern wins, and a pattern that matches a directory covers everything under it).
"""
from pathlib import Path
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]

# Dockerfile -> its ignore file. All three build from the repository root.
IMAGES = {
    "Dockerfile": ".dockerignore",
    "ci/Dockerfile": ".dockerignore",
    "deploy/huddles/Dockerfile": "deploy/huddles/Dockerfile.dockerignore",
}


def patterns(ignore_file):
    rules = []
    for line in (ROOT / ignore_file).read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        exclusion = line.startswith("!")
        pattern = line[1:].strip() if exclusion else line
        pattern = re.sub(r"/+", "/", pattern).strip("/")
        if pattern:
            rules.append((exclusion, compile_pattern(pattern)))
    return rules


def compile_pattern(pattern):
    regex, index = "", 0
    while index < len(pattern):
        char = pattern[index]
        if pattern.startswith("**/", index):
            regex += "(?:.*/)?"
            index += 3
            continue
        if pattern.startswith("**", index):
            regex += ".*"
            index += 2
            continue
        if char == "*":
            regex += "[^/]*"
        elif char == "?":
            regex += "[^/]"
        elif char == "[":
            end = pattern.index("]", index)
            regex += pattern[index:end + 1]
            index = end
        elif char == "\\" and index + 1 < len(pattern):
            index += 1
            regex += re.escape(pattern[index])
        else:
            regex += re.escape(char)
        index += 1
    return re.compile(regex + r"\Z")


def ignored(path, rules):
    parents = [str(Path(*Path(path).parts[:depth])) for depth in range(1, len(Path(path).parts))]
    result = False
    for exclusion, regex in rules:
        if regex.match(path) or any(regex.match(parent) for parent in parents):
            result = not exclusion
    return result


def copy_sources(dockerfile):
    text = re.sub(r"\\\n", " ", (ROOT / dockerfile).read_text())
    sources = []
    for line in text.splitlines():
        words = line.split()
        if not words or words[0].upper() not in {"COPY", "ADD"}:
            continue
        args = words[1:]
        if any(arg.startswith("--from=") for arg in args) or any(arg.startswith("<<") for arg in args):
            continue
        args = [arg for arg in args if not arg.startswith("--")]
        sources.extend(args[:-1])
    return sources


def tracked():
    output = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT, text=True)
    return [path for path in output.split("\0") if path]


class ImageContextTest(unittest.TestCase):
    def test_every_copied_source_survives_its_dockerignore(self):
        files = tracked()
        for dockerfile, ignore_file in IMAGES.items():
            rules = patterns(ignore_file)
            sources = copy_sources(dockerfile)
            self.assertTrue(sources, f"{dockerfile}: no COPY sources found")
            for source in sources:
                with self.subTest(dockerfile=dockerfile, source=source):
                    source = source.strip("/").removeprefix("./")
                    self.assertTrue((ROOT / source).exists(), f"{dockerfile} copies missing {source}")
                    self.assertFalse(ignored(source, rules), f"{ignore_file} hides {source}, which {dockerfile} copies")
                    if (ROOT / source).is_dir():
                        inside = [path for path in files if path.startswith(source + "/")]
                        kept = [path for path in inside if not ignored(path, rules)]
                        self.assertTrue(kept, f"{ignore_file} hides every file under {source}, which {dockerfile} copies")
            print(f"IMAGE CONTEXT: {dockerfile}: {len(sources)} COPY sources survive {ignore_file}")

    def test_credentials_and_scratch_files_stay_out_of_the_root_context(self):
        rules = patterns(".dockerignore")
        for path in ["gha-creds-0123456789abcdef.json", ".env", ".env.production", "err.log",
                     ".cargo-home/credentials.toml", ".scratch/release-inputs.x/Cargo.toml",
                     "target/debug/campfire", "parity/.seed/default/db/production.sqlite3",
                     "web/script/livekit-gateway/node_modules/x/index.js", ".git/config"]:
            with self.subTest(path=path):
                self.assertTrue(ignored(path, rules), f".dockerignore lets {path} into the build context")

    def test_the_matcher_follows_buildkit(self):
        rules = [(False, compile_pattern("*")), (True, compile_pattern("web/a.txt"))]
        self.assertFalse(ignored("web/a.txt", rules))
        self.assertTrue(ignored("web/b.txt", rules))
        rules = [(False, compile_pattern("crates/*/tests/golden"))]
        self.assertTrue(ignored("crates/views/tests/golden/x.json", rules))
        self.assertFalse(ignored("crates/views/tests/x.rs", rules))
        rules = [(False, compile_pattern("**/node_modules"))]
        self.assertTrue(ignored("node_modules/x", rules))
        self.assertTrue(ignored("a/b/node_modules/x", rules))
        rules = [(False, compile_pattern("*.md"))]
        self.assertTrue(ignored("README.md", rules))
        self.assertFalse(ignored("docs/a.md", rules))


if __name__ == "__main__":
    unittest.main()
