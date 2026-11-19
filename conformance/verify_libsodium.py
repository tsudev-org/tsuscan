#!/usr/bin/env python3
"""Kiểm conformance/vault-v1.json bằng libsodium CỦA HỆ ĐIỀU HÀNH (cài đặt C độc lập với RustCrypto) + hashlib.

Chạy: python3 -I conformance/verify_libsodium.py conformance/vault-v1.json
Kiểm: phong bì (XChaCha20-Poly1305 + đệm ISO 7816-4), luồng v1 (nonce tiền tố ‖ chỉ số ‖ cờ cuối, Padmé),
Argon2id (NFKC), HKDF-SHA256 của KEK khôi phục. BIP39 kiểm bằng vector chính thức trong test Rust.
"""
import ctypes, hashlib, hmac, json, sys, unicodedata

L = ctypes.CDLL('libsodium.so.23')
assert L.sodium_init() >= 0
L.sodium_version_string.restype = ctypes.c_char_p
print('libsodium', L.sodium_version_string().decode())
v = json.load(open(sys.argv[1], encoding='utf-8'))
key = bytes.fromhex(v['keyHex'])


def xdec(ct, aad, nonce):
    out = ctypes.create_string_buffer(max(len(ct), 1))
    olen = ctypes.c_ulonglong()
    rc = L.crypto_aead_xchacha20poly1305_ietf_decrypt(
        out, ctypes.byref(olen), None, ct, ctypes.c_ulonglong(len(ct)), aad, ctypes.c_ulonglong(len(aad)), nonce, key)
    return None if rc != 0 else out.raw[:olen.value]


def unpad(p, block):
    assert len(p) % block == 0 and len(p) > 0
    i = len(p) - 1
    while p[i] == 0:
        i -= 1
    assert p[i] == 0x80 and len(p) - i <= block
    return p[:i]


for e in v['envelopes']:
    s = bytes.fromhex(e['sealedHex'])
    hdr, nonce, ct = s[:4], s[4:28], s[28:]
    assert hdr == b'TS\x01' + bytes([e['kind']]), e['name']
    aad = hdr + e['context'].encode()
    p = xdec(ct, aad, nonce)
    assert p is not None, e['name']
    assert unpad(p, e['padBlock']).hex() == e['plaintextHex'], e['name']
    assert xdec(ct, aad + b'x', nonce) is None
    print('ĐẠT phong bì', e['name'])


def padme(n):
    if n < 2:
        return n
    e = n.bit_length() - 1
    s = e.bit_length()
    mask = (1 << (e - s)) - 1
    return (n + mask) & ~mask


CH = v['stream']['chunk'] + 16
for st in v['stream']['streams']:
    c = bytes.fromhex(st['ciphertextHex'])
    hdr = c[:24]
    assert hdr[:4] == b'TS\x01\x03' and hdr[23] == 16 and hdr[4:23].hex() == v['stream']['prefixHex']
    aad = hdr + st['context'].encode()
    body = c[24:]
    chunks = [body[i:i + CH] for i in range(0, len(body), CH)]
    logical = b''
    for i, ch in enumerate(chunks):
        last = i == len(chunks) - 1
        nonce = hdr[4:23] + i.to_bytes(4, 'big') + bytes([1 if last else 0])
        p = xdec(ch, aad, nonce)
        assert p is not None, (st['name'], i)
        if not last:  # cùng khối với cờ sai phải bị từ chối
            assert xdec(ch, aad, hdr[4:23] + i.to_bytes(4, 'big') + b'\x01') is None
        logical += p
    n = int.from_bytes(logical[:8], 'little')
    assert n == st['plaintextLen'] and len(logical) == padme(n + 8) == st['paddedLen']
    want = bytes((13 * i + 7) % 256 for i in range(n))
    assert logical[8:8 + n] == want and not any(logical[8 + n:])
    print('ĐẠT luồng', st['name'], len(chunks), 'khối')

a = v['argon2id']
pw = unicodedata.normalize('NFKC', a['passwordNfd']).encode()
assert pw != a['passwordNfd'].encode(), 'vector phải ở dạng NFD'
outk = ctypes.create_string_buffer(32)
assert a['p'] == 1  # libsodium chỉ hỗ trợ 1 luồng
rc = L.crypto_pwhash(outk, ctypes.c_ulonglong(32), pw, ctypes.c_ulonglong(len(pw)), bytes.fromhex(a['saltHex']),
                     ctypes.c_ulonglong(a['t']), ctypes.c_size_t(a['mKib'] * 1024), ctypes.c_int(2))
assert rc == 0 and outk.raw.hex() == a['keyHex']
print('ĐẠT argon2id')

r = v['recovery']
prk = hmac.new(b'\x00' * 32, bytes.fromhex(r['entropyHex']), hashlib.sha256).digest()
okm = hmac.new(prk, r['hkdfInfo'].encode() + b'\x01', hashlib.sha256).digest()
assert okm.hex() == r['keyHex'] and len(r['words'].split()) == 24
print('ĐẠT HKDF khôi phục')
