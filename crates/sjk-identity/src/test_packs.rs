//! Test packs (`PROTOCOL.md`, "Test packs"): PK3 files the owner sends to the players
//! they choose, through a hub that can never read them. This module is the cryptography
//! both ends share: the owner's tool locks a pack ([`encrypt_body`], [`seal_label`],
//! [`seal_envelope`]) and a chosen player's client opens it ([`open_envelope`],
//! [`open_label`], [`decrypt_body`]).
//!
//! A pack's own random key locks its body and label; an envelope seals that key to one
//! player's identity key, taken as an X25519 key (as `age` does with SSH keys), so only
//! that player's private key opens it. The hub keeps the locked body, the label and the
//! envelopes, none of which it can open.

use crate::keys::Identity;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use curve25519_dalek::montgomery::MontgomeryPoint;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Read, Seek, Write};

/// A pack's id: 32 lowercase hex digits.
const ID_LEN: usize = 32;
/// The body's header: magic, salt, chunk size, four zero bytes.
pub const HEADER_LEN: usize = 48;
const MAGIC: &[u8; 8] = b"SJKTPK1\0";
/// Plain bytes in each chunk of the body but the last.
pub const CHUNK: usize = 1 << 20;
/// A ChaCha20-Poly1305 tag.
const TAG: usize = 16;
/// An envelope's bytes: version, ephemeral X25519 key, sealed pack key.
const ENVELOPE_LEN: usize = 1 + 32 + 32 + TAG;
const ENVELOPE_VERSION: u8 = 1;
/// The longest label file name and note.
const FILE_MAX: usize = 64;
const NOTE_MAX: usize = 200;

/// Why a test pack could not be locked or opened.
#[derive(Debug)]
pub enum TestPackError {
    /// Reading or writing failed.
    Io(io::Error),
    /// Something is not what the format says, or does not open with this key.
    Invalid(String),
}

impl fmt::Display for TestPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Invalid(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for TestPackError {}

impl From<io::Error> for TestPackError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn invalid(why: impl Into<String>) -> TestPackError {
    TestPackError::Invalid(why.into())
}

/// The most bytes one download request asks for.
pub const RANGE_MAX: u64 = 32 * 1024 * 1024;
/// The bytes of each upload part but the last.
pub const PART: usize = 8 * 1024 * 1024;
/// The longest locked body.
pub const BODY_MAX: u64 = 2 * 1024 * 1024 * 1024;

/// One pack of `GET /v1/test-packs`: what this key may fetch and open.
#[derive(Clone, Deserialize, Eq, PartialEq)]
pub struct Listed {
    /// The pack's id ([`valid_id`]).
    pub id: String,
    /// The locked body's length.
    pub size: u64,
    /// The locked body's SHA-256, 64 lowercase hex digits.
    pub sha256: String,
    /// The locked label ([`open_label`]).
    pub label: String,
    /// This key's envelope: never written to disk.
    pub envelope: String,
}

impl fmt::Debug for Listed {
    // The envelope is left out of logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Listed({}, {} bytes)", self.id, self.size)
    }
}

/// The packs of a `GET /v1/test-packs` answer. A pack with a bad id, size, hash or
/// envelope, or an id listed twice, refuses the whole list.
pub fn listing(answer: serde_json::Value) -> Result<Vec<Listed>, crate::hub::HubError> {
    #[derive(Deserialize)]
    struct Answer {
        packs: Vec<Listed>,
    }
    let protocol = crate::hub::HubError::Protocol;
    let answer: Answer = serde_json::from_value(answer)
        .map_err(|error| protocol(format!("the test pack list: {error}")))?;
    let mut seen = std::collections::BTreeSet::new();
    for pack in &answer.packs {
        let fine = valid_id(&pack.id)
            && pack.size <= BODY_MAX
            && pack.size >= (HEADER_LEN + TAG) as u64
            && pack.sha256.len() == 64
            && pack
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && pack.envelope.len() == 108
            && pack.label.len() <= 1024
            && seen.insert(pack.id.as_str());
        if !fine {
            return Err(protocol(format!(
                "the test pack list has a bad pack {:?}",
                pack.id
            )));
        }
    }
    Ok(answer.packs)
}

/// What a pack's label says: the file name it is mounted under and a note.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Label {
    /// The file name the pack is mounted under ([`valid_file`]).
    pub file: String,
    /// The owner's note, up to 200 characters.
    pub note: String,
}

/// Whether `id` is a pack id: 32 lowercase hex digits.
pub fn valid_id(id: &str) -> bool {
    id.len() == ID_LEN
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether `file` may name a pack's file: 1 to 64 of `a`-`z`, `A`-`Z`, `0`-`9`, `_`,
/// `-` and `.`, ending in `.pk3`, not starting with `.`. Only such a name is ever
/// written to disk, so a label cannot reach outside the pack's folder.
pub fn valid_file(file: &str) -> bool {
    file.len() <= FILE_MAX
        && file.len() > ".pk3".len()
        && file.ends_with(".pk3")
        && !file.starts_with('.')
        && file
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

/// `N` random bytes from the operating system.
fn random<const N: usize>() -> Result<[u8; N], TestPackError> {
    crate::keys::random_bytes::<N>().map_err(|error| invalid(error.to_string()))
}

/// A new pack's key: 32 random bytes.
pub fn new_pack_key() -> Result<[u8; 32], TestPackError> {
    random()
}

/// A new pack's id: 16 random bytes as 32 lowercase hex digits.
pub fn new_pack_id() -> Result<String, TestPackError> {
    Ok(random::<16>()?
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// SHA-256 of `domain`, a zero byte, then `parts`: each key this module derives.
fn derive(domain: &str, parts: &[&[u8]]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(domain.as_bytes());
    hash.update([0]);
    for part in parts {
        hash.update(part);
    }
    hash.finalize().into()
}

fn cipher(key: &[u8; 32]) -> ChaCha20Poly1305 {
    ChaCha20Poly1305::new_from_slice(key).expect("a 32-byte key")
}

const ZERO_NONCE: [u8; 12] = [0; 12];

/// The pack key sealed to the identity key `recipient` (Ed25519, 32 bytes) for pack
/// `id`: an envelope, unpadded base64url (108 characters).
pub fn seal_envelope(
    pack_key: &[u8; 32],
    id: &str,
    recipient: &[u8; 32],
) -> Result<String, TestPackError> {
    let point = VerifyingKey::from_bytes(recipient)
        .map_err(|_| invalid("the recipient's key is not an Ed25519 public key"))?
        .to_montgomery();
    let ephemeral_secret: [u8; 32] = random()?;
    let ephemeral = MontgomeryPoint::mul_base_clamped(ephemeral_secret).to_bytes();
    let shared = point.mul_clamped(ephemeral_secret).to_bytes();
    if shared == [0; 32] {
        return Err(invalid("the recipient's key is a weak point"));
    }
    let wrap = derive(
        "SJK test pack envelope v1",
        &[&shared, &ephemeral, recipient],
    );
    let sealed = cipher(&wrap)
        .encrypt(
            &ZERO_NONCE.into(),
            Payload {
                msg: pack_key,
                aad: id.as_bytes(),
            },
        )
        .map_err(|_| invalid("sealing failed"))?;
    let mut bytes = Vec::with_capacity(ENVELOPE_LEN);
    bytes.push(ENVELOPE_VERSION);
    bytes.extend_from_slice(&ephemeral);
    bytes.extend_from_slice(&sealed);
    Ok(B64.encode(bytes))
}

/// The pack key in `envelope`, for pack `id`, opened with `identity`'s private key. Fails
/// for an envelope made for another key or pack, or changed.
pub fn open_envelope(
    identity: &Identity,
    id: &str,
    envelope: &str,
) -> Result<[u8; 32], TestPackError> {
    let bytes = B64
        .decode(envelope)
        .map_err(|_| invalid("the envelope is not base64url"))?;
    if bytes.len() != ENVELOPE_LEN || bytes[0] != ENVELOPE_VERSION {
        return Err(invalid("the envelope is not a version 1 envelope"));
    }
    let ephemeral: [u8; 32] = bytes[1..33].try_into().expect("32 bytes");
    let shared = identity.shared_secret(ephemeral);
    if shared == [0; 32] {
        return Err(invalid("the envelope's key is a weak point"));
    }
    let recipient = identity.public_key();
    let wrap = derive(
        "SJK test pack envelope v1",
        &[&shared, &ephemeral, &recipient],
    );
    let opened = cipher(&wrap)
        .decrypt(
            &ZERO_NONCE.into(),
            Payload {
                msg: &bytes[33..],
                aad: id.as_bytes(),
            },
        )
        .map_err(|_| invalid("the envelope is not for this key, or was changed"))?;
    opened
        .try_into()
        .map_err(|_| invalid("the envelope holds no 32-byte key"))
}

fn label_key(pack_key: &[u8; 32]) -> [u8; 32] {
    derive("SJK test pack label v1", &[pack_key])
}

/// `label` locked under `pack_key` for pack `id`, unpadded base64url.
pub fn seal_label(pack_key: &[u8; 32], id: &str, label: &Label) -> Result<String, TestPackError> {
    if !valid_file(&label.file) {
        return Err(invalid(format!("{:?} is not a pack file name", label.file)));
    }
    if label.note.chars().count() > NOTE_MAX {
        return Err(invalid("the note is over 200 characters"));
    }
    let text = serde_json::to_vec(label).map_err(|error| invalid(error.to_string()))?;
    let sealed = cipher(&label_key(pack_key))
        .encrypt(
            &ZERO_NONCE.into(),
            Payload {
                msg: &text,
                aad: id.as_bytes(),
            },
        )
        .map_err(|_| invalid("sealing failed"))?;
    Ok(B64.encode(sealed))
}

/// The label of pack `id`, opened with its `pack_key` and checked.
pub fn open_label(pack_key: &[u8; 32], id: &str, label: &str) -> Result<Label, TestPackError> {
    let bytes = B64
        .decode(label)
        .map_err(|_| invalid("the label is not base64url"))?;
    let text = cipher(&label_key(pack_key))
        .decrypt(
            &ZERO_NONCE.into(),
            Payload {
                msg: &bytes,
                aad: id.as_bytes(),
            },
        )
        .map_err(|_| invalid("the label does not open with this pack's key"))?;
    let label: Label =
        serde_json::from_slice(&text).map_err(|_| invalid("the label is not a label"))?;
    if !valid_file(&label.file) || label.note.chars().count() > NOTE_MAX {
        return Err(invalid("the label names no valid pack file"));
    }
    Ok(label)
}

fn body_key(pack_key: &[u8; 32], salt: &[u8]) -> [u8; 32] {
    derive("SJK test pack body v1", &[pack_key, salt])
}

fn nonce(index: u64, last: bool) -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[3..11].copy_from_slice(&index.to_be_bytes());
    bytes[11] = u8::from(last);
    bytes
}

/// Fill `buffer` as far as `reader` goes; the count read.
fn read_full(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

/// Blocks of `size` read one ahead, each with whether it is the last; an empty input
/// gives one empty last block.
struct Blocks<'a, R> {
    reader: &'a mut R,
    size: usize,
    next: Vec<u8>,
    ended: bool,
    done: bool,
}

impl<'a, R: Read> Blocks<'a, R> {
    fn new(reader: &'a mut R, size: usize) -> io::Result<Self> {
        let mut next = vec![0; size];
        let read = read_full(reader, &mut next)?;
        next.truncate(read);
        Ok(Self {
            reader,
            size,
            next,
            ended: read < size,
            done: false,
        })
    }

    fn take(&mut self) -> io::Result<Option<(Vec<u8>, bool)>> {
        if self.done {
            return Ok(None);
        }
        let mut ahead = Vec::new();
        if !self.ended {
            ahead = vec![0; self.size];
            let read = read_full(self.reader, &mut ahead)?;
            ahead.truncate(read);
            self.ended = read < self.size;
        }
        let last = ahead.is_empty();
        self.done = last;
        Ok(Some((std::mem::replace(&mut self.next, ahead), last)))
    }
}

/// Lock the PK3 read from `plain` into `locked` as pack `id`'s body under `pack_key`.
/// Answers the body's length.
pub fn encrypt_body(
    plain: &mut impl Read,
    locked: &mut impl Write,
    pack_key: &[u8; 32],
    id: &str,
) -> Result<u64, TestPackError> {
    if !valid_id(id) {
        return Err(invalid("not a pack id"));
    }
    let salt: [u8; 32] = random()?;
    let mut header = Vec::with_capacity(HEADER_LEN + ID_LEN);
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&salt);
    header.extend_from_slice(&(CHUNK as u32).to_be_bytes());
    header.extend_from_slice(&[0; 4]);
    locked.write_all(&header)?;
    header.extend_from_slice(id.as_bytes());
    let cipher = cipher(&body_key(pack_key, &salt));
    let mut written = HEADER_LEN as u64;
    let mut blocks = Blocks::new(plain, CHUNK)?;
    let mut index = 0_u64;
    while let Some((block, last)) = blocks.take()? {
        let sealed = cipher
            .encrypt(
                &nonce(index, last).into(),
                Payload {
                    msg: &block,
                    aad: &header,
                },
            )
            .map_err(|_| invalid("sealing failed"))?;
        locked.write_all(&sealed)?;
        written += sealed.len() as u64;
        index += 1;
    }
    Ok(written)
}

/// Open pack `id`'s body read from `locked` into `plain` with `pack_key`. Every chunk is
/// checked before it is written, and a body cut short or followed by more fails at its
/// end, so `plain` holds a whole PK3 only when this answers `Ok` (its length). Write to
/// a temporary file and keep it only then.
pub fn decrypt_body(
    locked: &mut impl Read,
    plain: &mut impl Write,
    pack_key: &[u8; 32],
    id: &str,
) -> Result<u64, TestPackError> {
    let mut header = vec![0; HEADER_LEN];
    if read_full(locked, &mut header)? < HEADER_LEN
        || &header[..8] != MAGIC
        || header[44..48] != [0; 4]
    {
        return Err(invalid("not a test pack body"));
    }
    let chunk = u32::from_be_bytes(header[40..44].try_into().expect("4 bytes")) as usize;
    if chunk != CHUNK {
        return Err(invalid("a body with another chunk size"));
    }
    let cipher = cipher(&body_key(pack_key, &header[8..40]));
    header.extend_from_slice(id.as_bytes());
    let mut blocks = Blocks::new(locked, CHUNK + TAG)?;
    let mut index = 0_u64;
    let mut written = 0_u64;
    while let Some((block, last)) = blocks.take()? {
        let opened = cipher
            .decrypt(
                &nonce(index, last).into(),
                Payload {
                    msg: &block,
                    aad: &header,
                },
            )
            .map_err(|_| invalid("the body was changed, cut short or is another pack's"))?;
        plain.write_all(&opened)?;
        written += opened.len() as u64;
        index += 1;
    }
    Ok(written)
}

/// A locked body read as its plain PK3, decrypting and checking each chunk only when a
/// read reaches it, so the PK3 is never written out and never held whole: what the game
/// mounts a test pack through. Opening checks the header and the last chunk (a body cut
/// at a chunk's end fails there); a chunk that does not check out fails the read that
/// reaches it.
pub struct BodyReader<R> {
    inner: R,
    cipher: ChaCha20Poly1305,
    /// The header, then the pack's id: every chunk's associated data.
    aad: Vec<u8>,
    chunks: u64,
    body_len: u64,
    plain_len: u64,
    position: u64,
    /// The chunk read last, by index, decrypted.
    cached: Option<(u64, Vec<u8>)>,
}

impl<R> fmt::Debug for BodyReader<R> {
    // Never prints the key or the plain bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BodyReader({} plain bytes)", self.plain_len)
    }
}

impl<R: Read + Seek> BodyReader<R> {
    /// The plain PK3 of pack `id`'s body read through `inner`, with `pack_key`.
    pub fn open(mut inner: R, pack_key: &[u8; 32], id: &str) -> Result<Self, TestPackError> {
        let body_len = inner.seek(io::SeekFrom::End(0))?;
        inner.seek(io::SeekFrom::Start(0))?;
        let mut header = vec![0; HEADER_LEN];
        if body_len < (HEADER_LEN + TAG) as u64
            || read_full(&mut inner, &mut header)? < HEADER_LEN
            || &header[..8] != MAGIC
            || header[44..48] != [0; 4]
            || u32::from_be_bytes(header[40..44].try_into().expect("4 bytes")) as usize != CHUNK
        {
            return Err(invalid("not a test pack body"));
        }
        let data = body_len - HEADER_LEN as u64;
        let sealed = (CHUNK + TAG) as u64;
        let chunks = data.div_ceil(sealed);
        if data - (chunks - 1) * sealed < TAG as u64 {
            return Err(invalid("the body was cut short"));
        }
        let cipher = cipher(&body_key(pack_key, &header[8..40]));
        header.extend_from_slice(id.as_bytes());
        let mut reader = Self {
            inner,
            cipher,
            aad: header,
            chunks,
            body_len,
            plain_len: data - chunks * TAG as u64,
            position: 0,
            cached: None,
        };
        reader
            .load(chunks - 1)
            .map_err(|error| invalid(error.to_string()))?;
        Ok(reader)
    }

    /// The plain PK3's length.
    pub fn plain_len(&self) -> u64 {
        self.plain_len
    }

    /// Chunk `index`, decrypted and checked, into the cache.
    fn load(&mut self, index: u64) -> io::Result<&[u8]> {
        if self
            .cached
            .as_ref()
            .is_none_or(|(cached, _)| *cached != index)
        {
            let sealed = (CHUNK + TAG) as u64;
            let start = HEADER_LEN as u64 + index * sealed;
            let length = sealed.min(self.body_len - start) as usize;
            let mut block = vec![0; length];
            self.inner.seek(io::SeekFrom::Start(start))?;
            self.inner.read_exact(&mut block)?;
            let last = index + 1 == self.chunks;
            let opened = self
                .cipher
                .decrypt(
                    &nonce(index, last).into(),
                    Payload {
                        msg: &block,
                        aad: &self.aad,
                    },
                )
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the test pack's body was changed or is another pack's",
                    )
                })?;
            self.cached = Some((index, opened));
        }
        Ok(self
            .cached
            .as_ref()
            .map(|(_, bytes)| bytes.as_slice())
            .unwrap_or_default())
    }
}

impl<R: Read + Seek> Read for BodyReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.plain_len || buffer.is_empty() {
            return Ok(0);
        }
        let index = self.position / CHUNK as u64;
        let offset = (self.position % CHUNK as u64) as usize;
        let chunk = self.load(index)?;
        let count = buffer.len().min(chunk.len().saturating_sub(offset));
        buffer[..count].copy_from_slice(&chunk[offset..offset + count]);
        self.position += count as u64;
        Ok(count)
    }
}

impl<R: Read + Seek> Seek for BodyReader<R> {
    fn seek(&mut self, to: io::SeekFrom) -> io::Result<u64> {
        let target = match to {
            io::SeekFrom::Start(at) => Some(at),
            io::SeekFrom::End(by) => self.plain_len.checked_add_signed(by),
            io::SeekFrom::Current(by) => self.position.checked_add_signed(by),
        };
        let target = target
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before the start"))?;
        self.position = target;
        Ok(target)
    }
}

/// The length of the body [`encrypt_body`] makes of a PK3 of `plain` bytes.
pub fn body_len(plain: u64) -> u64 {
    let chunks = plain.div_ceil(CHUNK as u64).max(1);
    HEADER_LEN as u64 + plain + chunks * TAG as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn an_envelope_opens_only_with_its_key_and_pack() {
        let tester = Identity::from_seed([7; 32]);
        let other = Identity::from_seed([8; 32]);
        let pack_key = new_pack_key().unwrap();
        let envelope = seal_envelope(&pack_key, ID, &tester.public_key()).unwrap();
        assert_eq!(envelope.len(), 108);
        assert_eq!(open_envelope(&tester, ID, &envelope).unwrap(), pack_key);
        assert!(open_envelope(&other, ID, &envelope).is_err());
        assert!(open_envelope(&tester, "fedcba9876543210fedcba9876543210", &envelope).is_err());
        // Two envelopes for the same key and pack differ (a new ephemeral key each).
        assert_ne!(
            seal_envelope(&pack_key, ID, &tester.public_key()).unwrap(),
            envelope
        );
        let mut bytes = B64.decode(&envelope).unwrap();
        bytes[40] ^= 1;
        assert!(open_envelope(&tester, ID, &B64.encode(&bytes)).is_err());
    }

    #[test]
    fn the_shared_secret_matches_libsodium_conversion() {
        // X25519(e, montgomery(P)) on the sealing side equals X25519(clamp(sha512(seed)),
        // E) on the opening side: the property every envelope relies on.
        let tester = Identity::from_seed([3; 32]);
        let secret = [9_u8; 32];
        let ephemeral = MontgomeryPoint::mul_base_clamped(secret).to_bytes();
        let point = VerifyingKey::from_bytes(&tester.public_key())
            .unwrap()
            .to_montgomery();
        assert_eq!(
            point.mul_clamped(secret).to_bytes(),
            tester.shared_secret(ephemeral)
        );
    }

    #[test]
    fn a_label_opens_with_its_pack_key_and_names_a_safe_file() {
        let pack_key = new_pack_key().unwrap();
        let label = Label {
            file: "anim_test_2.pk3".to_owned(),
            note: "emotes".to_owned(),
        };
        let sealed = seal_label(&pack_key, ID, &label).unwrap();
        assert_eq!(open_label(&pack_key, ID, &sealed).unwrap(), label);
        assert!(open_label(&new_pack_key().unwrap(), ID, &sealed).is_err());
        assert!(open_label(&pack_key, "fedcba9876543210fedcba9876543210", &sealed).is_err());
        for bad in [
            "../x.pk3",
            "a/b.pk3",
            ".hidden.pk3",
            "x.zip",
            ".pk3",
            "a b.pk3",
            "c:x.pk3",
        ] {
            assert!(!valid_file(bad), "{bad}");
        }
        assert!(valid_file("zz_Test-2.v3.pk3"));
    }

    #[test]
    fn bodies_of_every_shape_come_back_and_tampering_fails() {
        for size in [0, 1, CHUNK - 1, CHUNK, CHUNK + 1, 2 * CHUNK + 5] {
            let plain: Vec<u8> = (0..size).map(|i| (i * 7 % 251) as u8).collect();
            let pack_key = new_pack_key().unwrap();
            let mut body = Vec::new();
            let length = encrypt_body(&mut plain.as_slice(), &mut body, &pack_key, ID).unwrap();
            assert_eq!(length, body.len() as u64);
            assert_eq!(length, body_len(size as u64));
            let mut back = Vec::new();
            decrypt_body(&mut body.as_slice(), &mut back, &pack_key, ID).unwrap();
            assert_eq!(back, plain, "{size}");
            let fails = |bytes: &[u8], id: &str| {
                decrypt_body(&mut &bytes[..], &mut Vec::new(), &pack_key, id).is_err()
            };
            assert!(fails(&body, "fedcba9876543210fedcba9876543210"));
            let mut flipped = body.clone();
            let at = HEADER_LEN + (flipped.len() - HEADER_LEN) / 2;
            flipped[at] ^= 1;
            assert!(fails(&flipped, ID));
            let mut longer = body.clone();
            longer.push(0);
            assert!(fails(&longer, ID));
            if size > CHUNK {
                assert!(fails(&body[..HEADER_LEN + CHUNK + TAG], ID));
            }
            assert!(fails(&body[..HEADER_LEN], ID));
        }
    }

    #[test]
    fn the_body_reader_gives_the_plain_bytes_anywhere_and_refuses_damage() {
        use std::io::Cursor;
        for size in [0, 5, CHUNK, CHUNK + 3, 3 * CHUNK + 17] {
            let plain: Vec<u8> = (0..size).map(|i| (i * 13 % 251) as u8).collect();
            let pack_key = new_pack_key().unwrap();
            let mut body = Vec::new();
            encrypt_body(&mut plain.as_slice(), &mut body, &pack_key, ID).unwrap();
            let mut reader = BodyReader::open(Cursor::new(body.clone()), &pack_key, ID).unwrap();
            assert_eq!(reader.plain_len(), size as u64);
            let mut all = Vec::new();
            reader.read_to_end(&mut all).unwrap();
            assert_eq!(all, plain, "{size}");
            if size > CHUNK {
                // Across a chunk's end, from the back, then from the start again.
                let at = CHUNK as u64 - 2;
                reader.seek(io::SeekFrom::Start(at)).unwrap();
                let mut four = [0; 4];
                reader.read_exact(&mut four).unwrap();
                assert_eq!(four, plain[at as usize..at as usize + 4]);
                reader.seek(io::SeekFrom::End(-3)).unwrap();
                let mut tail = Vec::new();
                reader.read_to_end(&mut tail).unwrap();
                assert_eq!(tail, plain[size - 3..]);
                // A chunk changed in the middle fails only the read that reaches it; a
                // changed last chunk fails the opening.
                let mut flipped = body.clone();
                flipped[HEADER_LEN + CHUNK + TAG + 9] ^= 1;
                if size > 2 * CHUNK {
                    let mut damaged =
                        BodyReader::open(Cursor::new(flipped), &pack_key, ID).unwrap();
                    damaged.read_exact(&mut four).unwrap();
                    damaged.seek(io::SeekFrom::Start(CHUNK as u64 + 1)).unwrap();
                    assert!(damaged.read(&mut four).is_err());
                } else {
                    assert!(BodyReader::open(Cursor::new(flipped), &pack_key, ID).is_err());
                }
                // Cut at a chunk's end: the new last chunk is not marked last.
                let cut = body[..HEADER_LEN + CHUNK + TAG].to_vec();
                assert!(BodyReader::open(Cursor::new(cut), &pack_key, ID).is_err());
            }
            let wrong = new_pack_key().unwrap();
            assert!(BodyReader::open(Cursor::new(body.clone()), &wrong, ID).is_err());
            let other = "fedcba9876543210fedcba9876543210";
            assert!(BodyReader::open(Cursor::new(body), &pack_key, other).is_err());
        }
    }

    #[test]
    fn ids_are_32_lowercase_hex_digits() {
        let id = new_pack_id().unwrap();
        assert!(valid_id(&id));
        assert!(!valid_id(&id.to_uppercase().replace(char::is_numeric, "A")));
        assert!(!valid_id("0123"));
        assert!(!valid_id("0123456789abcdef0123456789abcdeg"));
    }
}
