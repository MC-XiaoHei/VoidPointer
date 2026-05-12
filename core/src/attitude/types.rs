use half::f16;

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
    fn identity_rotation() {
        let raw = SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let attitude = AttitudeData::from(raw);
        // 单位四元数: w=1 → roll/pitch/yaw 均为 0
        assert!((attitude.roll).abs() < 1e-6);
        assert!((attitude.pitch).abs() < 1e-6);
        assert!((attitude.yaw).abs() < 1e-6);
        assert!((attitude.w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn x_axis_rotation() {
        // x=1 表示绕 x 轴旋转 180 度
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
        // 所有分量为 0 导致 w_squared = 1 → w=1
        let raw = SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let a = AttitudeData::from(raw);
        assert!((a.w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn over_unit_quaternion_clamps_w() {
        // x^2 + y^2 + z^2 > 1 触发 w_squared <= 0 分支
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
        // w=0, sin_pitch = -2*z*x, 取 x=1.0, z=-1.0 → sin_pitch = 2 >= 1 → clamp
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
        // w=0, sin_pitch = -2*z*x, 取 x=-1.0, z=-1.0 → sin_pitch = -2 <= -1 → clamp
        let raw = SflpGameRotationRaw {
            x: f16::from_f32(-1.0).to_bits(),
            y: 0,
            z: f16::from_f32(-1.0).to_bits(),
        };
        let a = AttitudeData::from(raw);
        assert_eq!(a.pitch, -core::f32::consts::FRAC_PI_2);
    }
}
