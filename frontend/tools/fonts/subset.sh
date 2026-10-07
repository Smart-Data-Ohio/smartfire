#!/usr/bin/env bash
# Regenerates src/styles/fonts/*.woff2 from the upstream releases: Inter 4.1 (rsms/inter),
# JetBrains Mono 2.304 (JetBrains/JetBrainsMono), and the font presets people can pick in
# Settings: Atkinson Hyperlegible Next (googlefonts/atkinson-hyperlegible-next, pinned commit) and
# Source Serif 4.005 (adobe-fonts/source-serif). All are SIL OFL 1.1. Needs gh, unzip and python3;
# fonttools and brotli are installed into a throwaway venv under the work directory.
#
#   tools/fonts/subset.sh [work-dir]     (default: ~/.cache/smartfire-fonts)
set -euo pipefail

work="${1:-$HOME/.cache/smartfire-fonts}"
out="$(cd "$(dirname "$0")/../.." && pwd)/src/styles/fonts"
mkdir -p "$work"
cd "$work"

gh release download v4.1 --repo rsms/inter -p 'Inter-4.1.zip' --clobber
gh release download v2.304 --repo JetBrains/JetBrainsMono -p 'JetBrainsMono-2.304.zip' --clobber
unzip -o -q Inter-4.1.zip -d inter
unzip -o -q JetBrainsMono-2.304.zip -d jbm
gh release download 4.005R --repo adobe-fonts/source-serif -p 'source-serif-4.005_Desktop.zip' --clobber
unzip -o -q source-serif-4.005_Desktop.zip -d sserif
# Atkinson Hyperlegible Next publishes no releases; its variable fonts are fetched at one commit.
ahn_commit="${AHN_COMMIT:-7925f50f649b3813257faf2f4c0b381011f434f1}"
echo "Atkinson Hyperlegible Next at $ahn_commit"
for file in 'AtkinsonHyperlegibleNext[wght].ttf' 'AtkinsonHyperlegibleNext-Italic[wght].ttf'; do
  gh api -H 'Accept: application/vnd.github.raw' \
    "repos/googlefonts/atkinson-hyperlegible-next/contents/fonts/variable/$file?ref=$ahn_commit" > "$file"
done
gh api -H 'Accept: application/vnd.github.raw' \
  "repos/googlefonts/atkinson-hyperlegible-next/contents/OFL.txt?ref=$ahn_commit" > ahn-OFL.txt
[ -x venv/bin/pyftsubset ] || { python3 -m venv venv && venv/bin/pip -q install fonttools brotli; }

# Latin + Latin Extended (Google Fonts' ranges) plus arrows and the keyboard symbols Kbd shows.
range="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329"
range+=",U+2000-206F,U+20AC,U+2122,U+2190-21FF,U+2212,U+2215,U+FEFF,U+FFFD,U+0100-02BA"
range+=",U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+1E00-1E9F,U+1EF2-1EFF,U+20A0-20C0"
range+=",U+2113,U+2C60-2C7F,U+A720-A7FF,U+2318,U+2325,U+2303,U+238B,U+232B,U+23CE,U+2713,U+2715"

# The type scale uses weights 400-600 only; text sizes need only Inter's text optical size.
subset() { # input output features
  venv/bin/pyftsubset "$1" --unicodes="$range" --layout-features="$3" --flavor=woff2 --output-file="$2"
}
inter_features="kern,liga,calt,ccmp,locl,mark,mkmk,case,tnum,cv11,ss01,zero,frac"
venv/bin/fonttools varLib.instancer inter/InterVariable.ttf opsz=14 wght=400:600 -o inter.ttf -q
venv/bin/fonttools varLib.instancer inter/InterVariable-Italic.ttf opsz=14 wght=400:600 -o inter-italic.ttf -q
venv/bin/fonttools varLib.instancer 'jbm/fonts/variable/JetBrainsMono[wght].ttf' wght=400:600 -o jbm.ttf -q
subset inter.ttf "$out/inter-latin-var.woff2" "$inter_features"
subset inter-italic.ttf "$out/inter-latin-var-italic.woff2" "$inter_features"
subset jbm.ttf "$out/jetbrains-mono-latin-var.woff2" "kern,liga,calt,ccmp,locl,mark,mkmk,zero"

# Presets: the same weights and ranges; Source Serif pinned to its text optical size.
preset_features="kern,liga,calt,ccmp,locl,mark,mkmk,case,tnum"
venv/bin/fonttools varLib.instancer 'AtkinsonHyperlegibleNext[wght].ttf' wght=400:600 -o ahn.ttf -q
venv/bin/fonttools varLib.instancer 'AtkinsonHyperlegibleNext-Italic[wght].ttf' wght=400:600 -o ahn-italic.ttf -q
venv/bin/fonttools varLib.instancer sserif/source-serif-4.005_Desktop/VAR/SourceSerif4Variable-Roman.ttf opsz=16 wght=400:600 -o sserif.ttf -q
venv/bin/fonttools varLib.instancer sserif/source-serif-4.005_Desktop/VAR/SourceSerif4Variable-Italic.ttf opsz=16 wght=400:600 -o sserif-italic.ttf -q
subset ahn.ttf "$out/atkinson-hyperlegible-next-latin-var.woff2" "$preset_features"
subset ahn-italic.ttf "$out/atkinson-hyperlegible-next-latin-var-italic.woff2" "$preset_features"
subset sserif.ttf "$out/source-serif-4-latin-var.woff2" "$preset_features"
subset sserif-italic.ttf "$out/source-serif-4-latin-var-italic.woff2" "$preset_features"
cp inter/LICENSE.txt "$out/LICENSE-Inter.txt"
cp jbm/OFL.txt "$out/LICENSE-JetBrainsMono.txt"
cp ahn-OFL.txt "$out/LICENSE-AtkinsonHyperlegibleNext.txt"
cp sserif/source-serif-4.005_Desktop/LICENSE.md "$out/LICENSE-SourceSerif4.md"
ls -l "$out"
