"""Full proxy header comparison. Only the maintainer's nine named differences apply."""
import re
import uuid
from email.utils import parsedate_to_datetime

SECURITY = {
    'permissions-policy': 'camera=(self), display-capture=(self), microphone=(self), notifications=(self)',
    'referrer-policy': 'strict-origin-when-cross-origin',
    'x-content-type-options': 'nosniff',
    'x-frame-options': 'SAMEORIGIN',
    'x-permitted-cross-domain-policies': 'none',
    'x-xss-protection': '0',
}
PER_REQUEST = {'date', 'x-request-id', 'x-runtime'}

def checked(headers):
    result = {}
    for key, values in headers.items():
        name = key.lower()
        assert name not in result, ('duplicate header name', name)
        assert isinstance(values, list) and len(values) == 1, ('duplicate header values', name, values)
        assert isinstance(values[0], str) and '\n' not in values[0] and '\r' not in values[0], (name, values)
        value = values[0]
        if name == 'date':
            assert parsedate_to_datetime(value).tzinfo is not None
        elif name == 'x-request-id':
            assert str(uuid.UUID(value)) == value and uuid.UUID(value).version == 4, value
        elif name == 'x-runtime':
            assert re.fullmatch(r'\d+\.\d{6}', value), value
        result[name] = values
    return result

def compare_headers(actual, expected, streamed=False):
    actual, expected = checked(actual), checked(expected)
    for name in PER_REQUEST:
        assert (name in actual) == (name in expected), ('per-request header presence', name)
        actual.pop(name, None)
        expected.pop(name, None)
    if streamed:
        for name, value in SECURITY.items():
            assert actual.get(name) == [value], (name, actual.get(name))
            assert name not in expected, ('Rails streamed response gained a security header', name)
            actual.pop(name)
    assert actual == expected, (actual, expected)
