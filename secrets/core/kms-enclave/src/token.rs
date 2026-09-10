//! Wrapped-key tokens: the only serializable form of enclave key material.
//!
//! The header bytes double as the AEAD associated data: any header tamper fails
//! authentication, not just parsing.

use std::fmt;

use secrets_kms_domain::envelope_keys::{DekId, KekId};

const MAGIC: &[u8; 4] = b"OYK1";
const KIND_KEK: u8 = 0x01;
const KIND_DEK: u8 = 0x02;
const FORMAT_V1: u8 = 0x01;
pub(crate) const NONCE_LEN: usize = 12;
/// AES-256-GCM tag length — minimum valid ciphertext length.
const TAG_LEN: usize = 16;
const MAX_FIELD_LEN: usize = 512;
const MAX_CIPHERTEXT_LEN: usize = 4096;

/// Strict-decoding failures for wrapped tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    /// Input ended before a complete token was read.
    Truncated,
    /// Bytes remained after a complete token was read.
    TrailingBytes,
    /// Preamble is not `OYK1`.
    BadMagic,
    /// Kind byte does not match the expected token kind.
    WrongKind,
    /// Format version is not supported.
    UnsupportedFormat(u8),
    /// A length-prefixed field exceeds its hard cap.
    FieldTooLong,
    /// An identifier field is not valid UTF-8.
    InvalidUtf8,
    /// An identifier failed `KekId`/`DekId` domain validation.
    InvalidIdentifier,
    /// Ciphertext is shorter than the AEAD tag or exceeds its hard cap.
    BadCiphertextLength,
    /// KEK version field is zero (versions are 1-based).
    ZeroVersion,
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("token truncated"),
            Self::TrailingBytes => f.write_str("trailing bytes after token"),
            Self::BadMagic => f.write_str("bad token magic"),
            Self::WrongKind => f.write_str("wrong token kind"),
            Self::UnsupportedFormat(v) => write!(f, "unsupported token format {v}"),
            Self::FieldTooLong => f.write_str("token field exceeds cap"),
            Self::InvalidUtf8 => f.write_str("token identifier is not utf-8"),
            Self::InvalidIdentifier => f.write_str("token identifier failed validation"),
            Self::BadCiphertextLength => f.write_str("token ciphertext length invalid"),
            Self::ZeroVersion => f.write_str("token KEK version is zero"),
        }
    }
}

impl std::error::Error for TokenError {}

/// A KEK sealed under a per-cell sealing root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedKekToken {
    pub(crate) root_id: String,
    pub(crate) kek_id: KekId,
    pub(crate) kek_version: u32,
    pub(crate) nonce: [u8; NONCE_LEN],
    pub(crate) ciphertext: Vec<u8>,
}

impl WrappedKekToken {
    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn kek_id(&self) -> &KekId {
        &self.kek_id
    }

    pub fn kek_version(&self) -> u32 {
        self.kek_version
    }

    pub fn encode(&self) -> Vec<u8> {
        let header = kek_header(&self.root_id, &self.kek_id, self.kek_version);
        assemble(header, &self.nonce, &self.ciphertext)
    }

    /// Strict decode; rejects trailing bytes and malformed fields.
    pub fn decode(bytes: &[u8]) -> Result<Self, TokenError> {
        let mut cur = Cursor::new(bytes);
        cur.expect_preamble(KIND_KEK)?;
        let root_id = cur.read_string()?;
        let kek_id_raw = cur.read_string()?;
        let kek_version = cur.read_u32()?;
        if kek_version == 0 {
            return Err(TokenError::ZeroVersion);
        }
        let kek_id = KekId::new(kek_id_raw).map_err(|_| TokenError::InvalidIdentifier)?;
        let nonce = cur.read_nonce()?;
        let ciphertext = cur.read_ciphertext()?;
        cur.expect_end()?;
        Ok(Self {
            root_id,
            kek_id,
            kek_version,
            nonce,
            ciphertext,
        })
    }

    /// Header bytes (everything before the nonce) — the AEAD associated data.
    pub(crate) fn aad(&self) -> Vec<u8> {
        kek_header(&self.root_id, &self.kek_id, self.kek_version)
    }
}

/// A DEK sealed under a specific KEK version, which routes unwrapping to that version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedDek {
    pub(crate) kek_id: KekId,
    pub(crate) kek_version: u32,
    pub(crate) dek_id: DekId,
    pub(crate) nonce: [u8; NONCE_LEN],
    pub(crate) ciphertext: Vec<u8>,
}

impl WrappedDek {
    pub fn kek_id(&self) -> &KekId {
        &self.kek_id
    }

    pub fn kek_version(&self) -> u32 {
        self.kek_version
    }

    pub fn dek_id(&self) -> &DekId {
        &self.dek_id
    }

    pub fn encode(&self) -> Vec<u8> {
        let header = dek_header(&self.kek_id, self.kek_version, &self.dek_id);
        assemble(header, &self.nonce, &self.ciphertext)
    }

    /// Strict decode; rejects trailing bytes and malformed fields.
    pub fn decode(bytes: &[u8]) -> Result<Self, TokenError> {
        let mut cur = Cursor::new(bytes);
        cur.expect_preamble(KIND_DEK)?;
        let kek_id_raw = cur.read_string()?;
        let kek_version = cur.read_u32()?;
        if kek_version == 0 {
            return Err(TokenError::ZeroVersion);
        }
        let dek_id_raw = cur.read_string()?;
        let kek_id = KekId::new(kek_id_raw).map_err(|_| TokenError::InvalidIdentifier)?;
        let dek_id = DekId::new(dek_id_raw).map_err(|_| TokenError::InvalidIdentifier)?;
        let nonce = cur.read_nonce()?;
        let ciphertext = cur.read_ciphertext()?;
        cur.expect_end()?;
        Ok(Self {
            kek_id,
            kek_version,
            dek_id,
            nonce,
            ciphertext,
        })
    }

    /// Header bytes (everything before the nonce) — the AEAD associated data.
    pub(crate) fn aad(&self) -> Vec<u8> {
        dek_header(&self.kek_id, self.kek_version, &self.dek_id)
    }
}

pub(crate) fn kek_header(root_id: &str, kek_id: &KekId, kek_version: u32) -> Vec<u8> {
    let mut out = preamble(KIND_KEK);
    push_field(&mut out, root_id.as_bytes());
    push_field(&mut out, kek_id.value().as_bytes());
    out.extend_from_slice(&kek_version.to_be_bytes());
    out
}

pub(crate) fn dek_header(kek_id: &KekId, kek_version: u32, dek_id: &DekId) -> Vec<u8> {
    let mut out = preamble(KIND_DEK);
    push_field(&mut out, kek_id.value().as_bytes());
    out.extend_from_slice(&kek_version.to_be_bytes());
    push_field(&mut out, dek_id.value().as_bytes());
    out
}

fn preamble(kind: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(MAGIC);
    out.push(kind);
    out.push(FORMAT_V1);
    out
}

fn push_field(out: &mut Vec<u8>, bytes: &[u8]) {
    // Ids are validated <= MAX_FIELD_LEN < u16::MAX, so this cast cannot truncate.
    let len = bytes.len().min(MAX_FIELD_LEN) as u16;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&bytes[..len as usize]);
}

fn assemble(header: Vec<u8>, nonce: &[u8; NONCE_LEN], ciphertext: &[u8]) -> Vec<u8> {
    let mut out = header;
    out.extend_from_slice(nonce);
    let ct_len = ciphertext.len().min(MAX_CIPHERTEXT_LEN) as u16;
    out.extend_from_slice(&ct_len.to_be_bytes());
    out.extend_from_slice(&ciphertext[..ct_len as usize]);
    out
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], TokenError> {
        let end = self.pos.checked_add(n).ok_or(TokenError::Truncated)?;
        if end > self.bytes.len() {
            return Err(TokenError::Truncated);
        }
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn expect_preamble(&mut self, kind: u8) -> Result<(), TokenError> {
        let magic = self.take(MAGIC.len())?;
        if magic != MAGIC {
            return Err(TokenError::BadMagic);
        }
        let kind_byte = self.take(1)?[0];
        if kind_byte != kind {
            return Err(TokenError::WrongKind);
        }
        let format = self.take(1)?[0];
        if format != FORMAT_V1 {
            return Err(TokenError::UnsupportedFormat(format));
        }
        Ok(())
    }

    fn read_u16(&mut self) -> Result<u16, TokenError> {
        let raw = self.take(2)?;
        Ok(u16::from_be_bytes([raw[0], raw[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, TokenError> {
        let raw = self.take(4)?;
        Ok(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    fn read_string(&mut self) -> Result<String, TokenError> {
        let len = usize::from(self.read_u16()?);
        if len > MAX_FIELD_LEN {
            return Err(TokenError::FieldTooLong);
        }
        let raw = self.take(len)?;
        String::from_utf8(raw.to_vec()).map_err(|_| TokenError::InvalidUtf8)
    }

    fn read_nonce(&mut self) -> Result<[u8; NONCE_LEN], TokenError> {
        let raw = self.take(NONCE_LEN)?;
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(raw);
        Ok(nonce)
    }

    fn read_ciphertext(&mut self) -> Result<Vec<u8>, TokenError> {
        let len = usize::from(self.read_u16()?);
        if !(TAG_LEN..=MAX_CIPHERTEXT_LEN).contains(&len) {
            return Err(TokenError::BadCiphertextLength);
        }
        Ok(self.take(len)?.to_vec())
    }

    fn expect_end(&self) -> Result<(), TokenError> {
        if self.pos != self.bytes.len() {
            return Err(TokenError::TrailingBytes);
        }
        Ok(())
    }
}
