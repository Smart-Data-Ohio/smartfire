#!/usr/bin/env python3
# Configure the current pinned image after cutover, before release logout.
# Reads secrets only from stdin; all output is an allowlisted verification result.
import re

from once_configuration import configure

ALLOWED = {"APP_URL", "GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET", "GOOGLE_SIGN_IN_DOMAINS", "GOOGLE_PICKER_API_KEY", "GOOGLE_CLOUD_PROJECT_NUMBER"}


def validate(changes, host, settings):
    if not all(isinstance(v, str) and (v or k == "GOOGLE_SIGN_IN_DOMAINS") for k, v in changes.items()):
        raise RuntimeError("Unexpected or missing Google configuration")
    if changes["APP_URL"] != "https://" + host or not re.fullmatch(r"[0-9]+", changes["GOOGLE_CLOUD_PROJECT_NUMBER"]):
        raise RuntimeError("Unexpected deployment origin or project number")
    domains = changes["GOOGLE_SIGN_IN_DOMAINS"].split(",") if changes["GOOGLE_SIGN_IN_DOMAINS"] else []
    if not all(re.fullmatch(r"[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+", d) for d in domains):
        raise RuntimeError("Invalid sign-in domain allowlist")


if __name__ == "__main__":
    configure(ALLOWED, validate, "smartfire-google-config", configuration_name="Google configuration")
