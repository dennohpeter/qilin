use ethers::types::{H256, U256};

/// Small helper function to convert [U256] into [H256].
pub fn u256_to_h256_le(u: U256) -> H256 {
    let mut h = H256::default();
    u.to_little_endian(h.as_mut());
    h
}

/// Small helper function to convert [U256] into [H256].
pub fn u256_to_h256_be(u: U256) -> H256 {
    let mut h = H256::default();
    u.to_big_endian(h.as_mut());
    h
}

/// Small helper function to convert [H256] into [U256].
pub fn h256_to_u256_be(storage: H256) -> U256 {
    U256::from_big_endian(storage.as_bytes())
}

/// Small helper function to convert [H256] into [U256].
pub fn h256_to_u256_le(storage: H256) -> U256 {
    U256::from_little_endian(storage.as_bytes())
}

/// Small helper function to convert revm's [B160] into ethers's [H160].
#[inline]
pub fn b160_to_h160(b: revm::primitives::B160) -> ethers::types::H160 {
    ethers::types::H160(b.0)
}

/// Small helper function to convert ethers's [H160] into revm's [B160].
#[inline]
pub fn h160_to_b160(h: ethers::types::H160) -> revm::primitives::B160 {
    revm::primitives::B160(h.0)
}

/// Small helper function to convert revm's [B256] into ethers's [H256].
#[inline]
pub fn b256_to_h256(b: revm::primitives::B256) -> ethers::types::H256 {
    ethers::types::H256(b.0)
}

/// Small helper function to convert ether's [H256] into revm's [B256].
#[inline]
pub fn h256_to_b256(h: ethers::types::H256) -> revm::primitives::B256 {
    revm::primitives::B256(h.0)
}

/// Small helper function to convert ether's [U256] into revm's [U256].
#[inline]
pub fn u256_to_ru256(u: ethers::types::U256) -> revm::primitives::U256 {
    let mut buffer = [0u8; 32];
    u.to_little_endian(buffer.as_mut_slice());
    revm::primitives::U256::from_le_bytes(buffer)
}

/// Small helper function to convert revm's [U256] into ethers's [U256].
#[inline]
pub fn ru256_to_u256(u: revm::primitives::U256) -> ethers::types::U256 {
    ethers::types::U256::from_little_endian(&u.as_le_bytes())
}
