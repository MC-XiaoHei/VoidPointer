use crate::bindings::sflp_game_rotation_raw_t;

#[derive(Debug, Clone, Copy, Default)]
pub struct AttitudeData {
    /// radians
    pub roll: f32,
    /// radians
    pub pitch: f32,
    /// radians
    pub yaw: f32,
    /// quaternion scalar part
    pub w: f32,
    /// quaternion x
    pub x: f32,
    /// quaternion y
    pub y: f32,
    /// quaternion z
    pub z: f32,
}

impl From<sflp_game_rotation_raw_t> for AttitudeData {
    fn from(raw: sflp_game_rotation_raw_t) -> Self {
        let x = f16_bits_to_f32(raw.x);
        let y = f16_bits_to_f32(raw.y);
        let z = f16_bits_to_f32(raw.z);
        let ww = 1.0_f32 - x * x - y * y - z * z;
        let w = if ww > 0.0 { libm::sqrtf(ww) } else { 0.0 };
        let roll = libm::atan2f(2.0 * (w * x + y * z), 1.0 - 2.0 * (x * x + y * y));
        let sinp = 2.0 * (w * y - z * x);
        let pitch = if sinp >= 1.0 {
            core::f32::consts::FRAC_PI_2
        } else if sinp <= -1.0 {
            -core::f32::consts::FRAC_PI_2
        } else {
            libm::asinf(sinp)
        };
        let yaw = libm::atan2f(2.0 * (w * z + x * y), 1.0 - 2.0 * (y * y + z * z));
        Self {
            roll,
            pitch,
            yaw,
            w,
            x,
            y,
            z,
        }
    }
}

fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 0x1) as u32;
    let exp = ((bits >> 10) & 0x1f) as u32;
    let frac = (bits & 0x03ff) as u32;
    let f_bits: u32 = if exp == 0 {
        if frac == 0 {
            sign << 31
        } else {
            let mut frac_norm = frac;
            let mut exp_shift = 0u32;
            while (frac_norm & 0x0400) == 0 {
                frac_norm <<= 1;
                exp_shift += 1;
            }
            frac_norm &= 0x03ff;
            let exp32 = 127 - 15 - exp_shift + 1;
            (sign << 31) | (exp32 << 23) | (frac_norm << 13)
        }
    } else if exp == 0x1f {
        (sign << 31) | (0xff << 23) | (frac << 13)
    } else {
        let exp32 = exp + (127 - 15);
        (sign << 31) | (exp32 << 23) | (frac << 13)
    };
    f32::from_bits(f_bits)
}
