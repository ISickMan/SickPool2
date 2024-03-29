use crypto_bigint::U256;

pub const CURRENT_VERSION: u32 = 1;
pub const OLDEST_COMPATIBLE_VERSION: u32 = 1;

pub const DEFAULT_STRATUM_CREATE_POOL_PORT: u16 = 9999;

// one share is this many share units (SUI) :)
// this is the lowest payout value: coin / units
const PAYOUT_DECIMAL_PERCISION: u32 = 6;
pub const PPLNS_SHARE_UNITS: u64 = 10u64.pow(PAYOUT_DECIMAL_PERCISION);
pub const PPLNS_SHARE_UNITS_256: U256 = U256::from_u64(PPLNS_SHARE_UNITS);

pub const PPLNS_DIFF_MULTIPLIER: u64 = 5;
pub const MAX_RETARGET_FACTOR : u64 = 2;

// pub const DEV_ADDRESS_BTC_STR: &str = "bc1q3k7q92qf3hmpdpekz4t9r2e3tszy2g4gv9gwea";
pub const DEV_ADDRESS_BTC_STR: &'static str = "bcrt1q9ude4m7uetjdwv5ud5h6qn7740ret7sznanxch";

// graffiti term borrowed from iron fish (?), very nice.
pub const GENERATION_GRAFFITI : &'static [u8; 32] = b"Mined the right way on P3Pool ||";
