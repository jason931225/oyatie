//! Page-locked, zeroize-on-drop key buffer.
//!
//! `MlockedKey` is the only container in which raw key bytes exist inside the
//! enclave kernel. The buffer is heap-allocated (stable address), pinned with
//! `mlock(2)` so the kernel never writes the page to swap, zeroized on drop,
//! and only then `munlock(2)`ed. Construction fails closed if `mlock` refuses
//! — unpinned key material is never accepted.

use std::fmt;

use zeroize::Zeroize;

use crate::EnclaveError;

pub(crate) const KEY_LEN: usize = 32;

/// 256-bit key in an `mlock`ed, zeroize-on-drop heap buffer.
pub(crate) struct MlockedKey {
    bytes: Box<[u8; KEY_LEN]>,
}

impl MlockedKey {
    fn new_zeroed() -> Result<Self, EnclaveError> {
        let bytes = Box::new([0u8; KEY_LEN]);
        // SAFETY: `bytes` is a live heap allocation of exactly KEY_LEN bytes;
        // the region stays valid until `Drop`, which munlocks the same range.
        let rc = unsafe { libc::mlock(bytes.as_ptr().cast(), KEY_LEN) };
        if rc != 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return Err(EnclaveError::MemoryLockFailed { errno });
        }
        Ok(Self { bytes })
    }

    pub(crate) fn generate() -> Result<Self, EnclaveError> {
        let mut key = Self::new_zeroed()?;
        aws_lc_rs::rand::fill(key.bytes.as_mut_slice())
            .map_err(|_| EnclaveError::RandomSourceFailed)?;
        Ok(key)
    }

    /// Move externally produced key bytes into locked memory, zeroizing the
    /// source. Ingress door only — there is no inverse.
    pub(crate) fn from_bytes(mut src: [u8; KEY_LEN]) -> Result<Self, EnclaveError> {
        let mut key = Self::new_zeroed()?;
        key.bytes.copy_from_slice(&src);
        src.zeroize();
        Ok(key)
    }

    pub(crate) fn expose(&self) -> &[u8; KEY_LEN] {
        &self.bytes
    }
}

impl Drop for MlockedKey {
    fn drop(&mut self) {
        self.bytes.zeroize();
        // SAFETY: same pointer/length pinned in `new_zeroed`. munlock after
        // zeroize so the page is scrubbed before it becomes swappable again.
        unsafe {
            libc::munlock(self.bytes.as_ptr().cast(), KEY_LEN);
        }
    }
}

impl fmt::Debug for MlockedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MlockedKey([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::MlockedKey;
    use std::marker::PhantomData;

    // Autoref specialization: `detect()` resolves to the inherent method (true)
    // only when `T: Clone`, else to the trait fallback (false).
    struct CloneProbe<T>(PhantomData<T>);

    impl<T: Clone> CloneProbe<T> {
        fn detect(&self) -> bool {
            true
        }
    }

    trait NotCloneFallback {
        fn detect(&self) -> bool {
            false
        }
    }

    impl<T> NotCloneFallback for CloneProbe<T> {}

    #[test]
    fn mlocked_key_is_not_clone() {
        assert!(
            !CloneProbe::<MlockedKey>(PhantomData).detect(),
            "MlockedKey must NOT implement Clone: a derived clone copies the key \
             into an unpinned, un-mlocked allocation"
        );
        assert!(CloneProbe::<String>(PhantomData).detect());
    }
}
