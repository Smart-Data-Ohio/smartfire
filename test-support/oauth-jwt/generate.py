#!/usr/bin/env python3
"""Regenerate public test identities with OpenSSL; none are production credentials."""
import argparse
import base64
from datetime import datetime
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent


def run(*args, input=None):
    return subprocess.check_output(["openssl", *map(str, args)], input=input, stderr=subprocess.DEVNULL)


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


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--only", choices=["rsa", "ec", "rsa-1024"])
args = parser.parse_args()
with tempfile.TemporaryDirectory() as directory:
    for name, options in [("rsa", ["RSA", "-pkeyopt", "rsa_keygen_bits:2048"]),
                          ("ec", ["EC", "-pkeyopt", "ec_paramgen_curve:P-256"]),
                          ("rsa-1024", ["RSA", "-pkeyopt", "rsa_keygen_bits:1024"])]:
        if args.only and name != args.only:
            continue
        pem = Path(directory) / f"{name}.pem"
        run("genpkey", "-algorithm", *options, "-out", pem)
        if name != "rsa-1024":
            (ROOT / f"{name}.der").write_bytes(run("pkcs8", "-topk8", "-nocrypt", "-in", pem, "-outform", "DER"))
        spki = run("pkey", "-in", pem, "-pubout", "-outform", "DER")
        _, sequence, _ = tlv(spki)
        _, _, end = tlv(sequence)
        _, bitstring, _ = tlv(sequence, end)
        public = bitstring[1:]
        if name.startswith("rsa"):
            _, sequence, _ = tlv(public)
            _, n, end = tlv(sequence)
            _, e, _ = tlv(sequence, end)
            key = dict(kty="RSA", kid="rsa", alg="RS256", n=b64(n.lstrip(b"\0")), e=b64(e.lstrip(b"\0")))
            if name == "rsa":
                (ROOT / "rsa-public.pem").write_bytes(run("pkey", "-in", pem, "-pubout"))
            else:
                now = int(datetime.fromisoformat("2026-10-13T16:00:00+00:00").timestamp())
                client = "https://client.example/metadata.json"
                claims = dict(iss=client, sub=client, aud="https://smartfire.example", exp=now+120, iat=now, jti="small RSA")
                message = (b64(json.dumps(dict(alg="RS256", kid="rsa")).encode()) + "." + b64(json.dumps(claims).encode())).encode()
                signed = run("dgst", "-sha256", "-sign", pem, input=message)
                (ROOT / f"{name}.jwt").write_text(message.decode() + "." + b64(signed) + "\n")
        else:
            key = dict(kty="EC", kid="ec", alg="ES256", crv="P-256", x=b64(public[1:33]), y=b64(public[33:]))
        (ROOT / f"{name}.jwks.json").write_text(json.dumps(dict(keys=[key]), indent=2) + "\n")
