use libsodium_sys::{
    crypto_box_MACBYTES, crypto_box_NONCEBYTES, crypto_box_PUBLICKEYBYTES,
    crypto_box_SECRETKEYBYTES, crypto_box_easy, crypto_kx_keypair, crypto_kx_seed_keypair,
    crypto_scalarmult_base, randombytes_buf, sodium_init, crypto_box_open_easy,
};

pub const PUBLIC_KEY_BYTES: usize = crypto_box_PUBLICKEYBYTES as usize;
pub const SECRET_KEY_BYTES: usize = crypto_box_SECRETKEYBYTES as usize;
pub const NONCE_BYTES: usize = crypto_box_NONCEBYTES as usize;
const MAC_BYTES: usize = crypto_box_MACBYTES as usize;

pub struct PublicKey([u8; PUBLIC_KEY_BYTES]);

impl PublicKey {
    pub fn key(&self) -> &[u8; PUBLIC_KEY_BYTES] {
        return &self.0;
    }

    pub fn new(k: [u8; PUBLIC_KEY_BYTES]) -> Self {
        Self(k)
    }
}

pub struct SecretKey([u8; SECRET_KEY_BYTES]);

impl SecretKey {
    pub fn key(&self) -> &[u8; SECRET_KEY_BYTES] {
        return &self.0;
    }

    pub fn new(k: [u8; SECRET_KEY_BYTES]) -> Self {
        Self(k)
    }

    pub fn public_key(&self) -> PublicKey {
        let mut pk = PublicKey([0u8; PUBLIC_KEY_BYTES]);

        unsafe {
            crypto_scalarmult_base(pk.0.as_mut_ptr(), self.0.as_ptr());
        }

        pk
    }
}

pub struct Nonce([u8; NONCE_BYTES]);

impl Nonce {
    pub fn new(n: [u8; NONCE_BYTES]) -> Self {
        Self(n)
    }

    pub fn value(&self) -> &[u8; NONCE_BYTES] {
        &self.0
    }
}

pub fn init() {
    unsafe {
        sodium_init();
    }
}

pub fn gen_keypair() -> (PublicKey, SecretKey) {
    let mut pk = PublicKey([0u8; PUBLIC_KEY_BYTES]);
    let mut sk = SecretKey([0u8; SECRET_KEY_BYTES]);

    unsafe {
        crypto_kx_keypair(pk.0.as_mut_ptr(), sk.0.as_mut_ptr());
    }

    (pk, sk)
}

pub fn keypair_from_seed(seed: u8) -> (PublicKey, SecretKey) {
    let mut pk = PublicKey([0u8; PUBLIC_KEY_BYTES]);
    let mut sk = SecretKey([0u8; SECRET_KEY_BYTES]);

    unsafe {
        crypto_kx_seed_keypair(pk.0.as_mut_ptr(), sk.0.as_mut_ptr(), &seed as *const u8);
    }

    (pk, sk)
}

pub fn easy(m: &[u8], n: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Vec<u8> {
    let clen = m.len() + MAC_BYTES;
    let mut c = Vec::with_capacity(clen);

    unsafe {
        c.set_len(clen);
        crypto_box_easy(
            c.as_mut_ptr(),
            m.as_ptr(),
            m.len() as u64,
            n.0.as_ptr(),
            pk.0.as_ptr(),
            sk.0.as_ptr(),
        );
    }

    c
}

pub fn open(c: &[u8], n: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Result<Vec<u8>, ()> {
    if c.len() < MAC_BYTES {
        return Err(());
    }
    let mlen = c.len() - MAC_BYTES;
    let mut m = Vec::with_capacity(mlen);
    let ret = unsafe {
        m.set_len(mlen);
        crypto_box_open_easy(
            m.as_mut_ptr(),
            c.as_ptr(),
            c.len() as u64,
            n.0.as_ptr(),
            pk.0.as_ptr(),
            sk.0.as_ptr(),
        )
    };
    if ret == 0 {
        Ok(m)
    } else {
        Err(())
    }
}

pub fn gen_nonce() -> Nonce {
    let mut n = Nonce([0u8; NONCE_BYTES]);

    unsafe {
        randombytes_buf(n.0.as_mut_ptr() as *mut libc::c_void, NONCE_BYTES);
    }

    n
}
