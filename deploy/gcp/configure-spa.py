#!/usr/bin/env python3
"""Set the SPA switches on the current pinned image without replacing other settings."""

from once_configuration import configure

ALLOWED = {"SPA_ENABLED", "SPA_DEFAULT"}


def validate(changes, host, settings):
    if (changes["SPA_ENABLED"], changes["SPA_DEFAULT"]) not in {
        ("0", "classic"), ("1", "classic"), ("1", "next"),
    }:
        raise RuntimeError("Unexpected SPA configuration values")
    if settings.get("autoUpdate") is not False:
        raise RuntimeError("Automatic image updates must already be disabled")


if __name__ == "__main__":
    configure(ALLOWED, validate, "smartfire-spa-config", configuration_name="SPA configuration", include_plan_values=True)
