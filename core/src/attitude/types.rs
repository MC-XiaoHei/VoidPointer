use half::f16;

const QUAT_NORM_SQ_MIN: f32 = 0.5;
const QUAT_NORM_SQ_MAX: f32 = 1.5;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SflpGameRotationRaw {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AttitudeData {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl AttitudeData {
    pub fn is_valid(&self) -> bool {
        let all_finite = self.roll.is_finite()
            && self.pitch.is_finite()
            && self.yaw.is_finite()
            && self.w.is_finite()
            && self.x.is_finite()
            && self.y.is_finite()
            && self.z.is_finite();
        if all_finite {
            let norm_sq = self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z;
            norm_sq >= QUAT_NORM_SQ_MIN && norm_sq <= QUAT_NORM_SQ_MAX
        } else {
            false
        }
    }
}

impl From<SflpGameRotationRaw> for AttitudeData {
    fn from(raw: SflpGameRotationRaw) -> Self {
        let x = f16::from_bits(raw.x).to_f32();
        let y = f16::from_bits(raw.y).to_f32();
        let z = f16::from_bits(raw.z).to_f32();

        let w_squared = 1.0_f32 - x * x - y * y - z * z;
        let w = if w_squared > 0.0 {
            libm::sqrtf(w_squared)
        } else {
            0.0
        };

        let sin_roll_cos_pitch = 2.0 * (w * x + y * z);
        let cos_roll_cos_pitch = 1.0 - 2.0 * (x * x + y * y);
        let roll = libm::atan2f(sin_roll_cos_pitch, cos_roll_cos_pitch);

        let sin_pitch = 2.0 * (w * y - z * x);
        let pitch = if sin_pitch >= 1.0 {
            core::f32::consts::FRAC_PI_2
        } else if sin_pitch <= -1.0 {
            -core::f32::consts::FRAC_PI_2
        } else {
            libm::asinf(sin_pitch)
        };

        let sin_yaw_cos_pitch = 2.0 * (w * z + x * y);
        let cos_yaw_cos_pitch = 1.0 - 2.0 * (y * y + z * z);
        let yaw = libm::atan2f(sin_yaw_cos_pitch, cos_yaw_cos_pitch);

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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn valid_identity_rotation() {
        let raw = SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let attitude = AttitudeData::from(raw);
        assert!(attitude.is_valid());
    }

    #[test]
    fn valid_normal_rotation() {
        let raw = SflpGameRotationRaw {
            x: f16::from_f32(0.5).to_bits(),
            y: 0,
            z: 0,
        };
        let attitude = AttitudeData::from(raw);
        assert!(attitude.is_valid());
    }

    #[test]
    fn invalid_nan_is_caught() {
        let attitude = AttitudeData {
            roll: f32::NAN,
            ..AttitudeData::default()
        };
        assert!(!attitude.is_valid());
    }

    #[test]
    fn invalid_infinite_is_caught() {
        let attitude = AttitudeData {
            pitch: f32::INFINITY,
            ..AttitudeData::default()
        };
        assert!(!attitude.is_valid());
    }

    #[test]
    fn invalid_negative_infinite_is_caught() {
        let attitude = AttitudeData {
            yaw: f32::NEG_INFINITY,
            ..AttitudeData::default()
        };
        assert!(!attitude.is_valid());
    }

    #[test]
    fn invalid_extreme_norm_is_caught() {
        let attitude = AttitudeData {
            w: 100.0,
            ..AttitudeData::default()
        };
        assert!(!attitude.is_valid());
    }

    #[test]
    fn identity_rotation() {
        let raw = SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let attitude = AttitudeData::from(raw);
        assert!((attitude.roll).abs() < 1e-6);
        assert!((attitude.pitch).abs() < 1e-6);
        assert!((attitude.yaw).abs() < 1e-6);
        assert!((attitude.w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn x_axis_rotation() {
        let raw = SflpGameRotationRaw {
            x: f16::from_f32(1.0).to_bits(),
            y: 0,
            z: 0,
        };
        let attitude = AttitudeData::from(raw);
        assert!((attitude.roll).abs() > 1e-6);
    }

    #[test]
    fn zero_quaternion_produces_zero_w() {
        let raw = SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let a = AttitudeData::from(raw);
        assert!((a.w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn over_unit_quaternion_clamps_w() {
        let raw = SflpGameRotationRaw {
            x: f16::from_f32(1.0).to_bits(),
            y: f16::from_f32(0.5).to_bits(),
            z: 0,
        };
        let a = AttitudeData::from(raw);
        assert_eq!(a.w, 0.0);
    }

    #[test]
    fn sin_pitch_overflow_clamps() {
        let raw = SflpGameRotationRaw {
            x: f16::from_f32(1.0).to_bits(),
            y: 0,
            z: f16::from_f32(-1.0).to_bits(),
        };
        let a = AttitudeData::from(raw);
        assert_eq!(a.pitch, core::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn sin_pitch_underflow_clamps() {
        let raw = SflpGameRotationRaw {
            x: f16::from_f32(-1.0).to_bits(),
            y: 0,
            z: f16::from_f32(-1.0).to_bits(),
        };
        let a = AttitudeData::from(raw);
        assert_eq!(a.pitch, -core::f32::consts::FRAC_PI_2);
    }
}
