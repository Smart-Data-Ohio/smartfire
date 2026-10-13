#!/usr/bin/env python3
"""Regenerate public test identities with OpenSSL; none are production credentials."""
import base64
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent


def run(*args):
    return subprocess.check_output(["openssl", *map(str, args)], stderr=subprocess.DEVNULL)


def tlv(data, offset=0):
    tag, length = data[offset:offset + 2]
    start = offset + 2
    if length & 128:
        count = length & 127
        length = int.from_bytes(data[start:start + count], "big")
        start += count
    return tag, data[start:start + length], start + length


def b64(data):
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


with tempfile.TemporaryDirectory() as directory:
    for name, options in [("rsa", ["RSA", "-pkeyopt", "rsa_keygen_bits:2048"]),
                          ("ec", ["EC", "-pkeyopt", "ec_paramgen_curve:P-256"])]:
        pem = Path(directory) / f"{name}.pem"
        run("genpkey", "-algorithm", *options, "-out", pem)
        (ROOT / f"{name}.der").write_bytes(run("pkcs8", "-topk8", "-nocrypt", "-in", pem, "-outform", "DER"))
        spki = run("pkey", "-in", pem, "-pubout", "-outform", "DER")
        _, sequence, _ = tlv(spki)
        _, _, end = tlv(sequence)
        _, bitstring, _ = tlv(sequence, end)
        public = bitstring[1:]
        if name == "rsa":
            _, sequence, _ = tlv(public)
            _, n, end = tlv(sequence)
            _, e, _ = tlv(sequence, end)
            key = dict(kty="RSA", kid="rsa", alg="RS256", n=b64(n.lstrip(b"\0")), e=b64(e.lstrip(b"\0")))
            (ROOT / "rsa-public.pem").write_bytes(run("pkey", "-in", pem, "-pubout"))
        else:
            key = dict(kty="EC", kid="ec", alg="ES256", crv="P-256", x=b64(public[1:33]), y=b64(public[33:]))
        (ROOT / f"{name}.jwks.json").write_text(json.dumps(dict(keys=[key]), indent=2) + "\n")
