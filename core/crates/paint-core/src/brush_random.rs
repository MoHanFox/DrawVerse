pub(crate) fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
pub(crate) fn unit(key: u64, index: u32, lane: u64) -> f64 {
    let value = mix(key ^ mix(u64::from(index)) ^ mix(lane.wrapping_add(0xd1b54a32d192ed03)));
    ((value >> 40) as u32) as f64 / 16777216.
}
