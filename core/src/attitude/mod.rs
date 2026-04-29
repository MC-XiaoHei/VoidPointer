use crate::attitude::types::AttitudeData;
use crate::bindings::{c_read_sflp_game_rotation_raw, sflp_game_rotation_raw_t};

pub mod types;

#[inline]
pub fn get_current_attitude() -> Option<AttitudeData> {
    let mut data = sflp_game_rotation_raw_t { x: 0, y: 0, z: 0 };
    let success = unsafe { c_read_sflp_game_rotation_raw(&mut data) };
    if success {
        Some(AttitudeData::from(data))
    } else {
        None
    }
}
