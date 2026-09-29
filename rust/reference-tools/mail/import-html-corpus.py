#!/usr/bin/env python3
"""Extract redacted real-email HTML structures; never deliver these corpus messages."""
import email
import email.policy
import hashlib
import json
import re
import sys
import tarfile
from pathlib import Path


def redact_text(value):
    # Preserve entities and whitespace, which affect Nokogiri's text output. Replace prose
    # and identifiers; copyright in the original message text remains with its sender.
    return ''.join(
        part if re.fullmatch(r'&(?:#[xX]?[0-9a-fA-F]+|[a-zA-Z]+);', part)
        else ''.join('x' if char.isalnum() else char for char in part)
        for part in re.split(r'(&(?:#[xX]?[0-9a-fA-F]+|[a-zA-Z]+);)', value)
    )


def redact_html(value):
    tokens = re.split(r'(<[^>]*>)', value)
    result = []
    for token in tokens:
        if not token.startswith('<'):
            result.append(redact_text(token))
        elif token.startswith('<!--') or token.startswith('<!'):
            result.append('<!---->')
        else:
            # Keep the original tag/attribute names and malformed nesting, not attribute
            # content such as tracking URLs. No HTML parser is used during extraction.
            result.append(re.sub(
                r'(=\s*)("[^"]*"|\x27[^\x27]*\x27|[^\s>]+)',
                lambda match: match[1] + redact_text(match[2]), token,
            ))
    return ''.join(result)


cases = []
for filename in sys.argv[1:]:
    archive = Path(filename)
    checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
    count = 0
    with tarfile.open(archive, 'r:bz2') as mails:
        for member in sorted(mails.getmembers(), key=lambda item: item.name):
            if not member.isfile():
                continue
            message = email.message_from_binary_file(mails.extractfile(member), policy=email.policy.default)
            for index, part in enumerate(message.walk()):
                if part.get_content_type() != 'text/html':
                    continue
                content = part.get_payload(decode=True)
                if content is None or not 500 <= len(content) <= 40000:
                    continue
                html = content.decode(part.get_content_charset() or 'utf-8', errors='replace')
                cases.append({
                    'source': member.name,
                    'part': index,
                    'archive_sha256': checksum,
                    'html': redact_html(html),
                })
                count += 1
                break
            if count == 16:
                break
    if count != 16:
        raise SystemExit(f'{archive}: expected 16 HTML parts, found {count}')
output = Path(__file__).with_name('corpus') / 'public-email-html.json'
output.parent.mkdir(exist_ok=True)
output.write_text(json.dumps(cases, indent=2, ensure_ascii=False) + '\n')
print(f'Imported {len(cases)} redacted real-email HTML structures')
