use half::f16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SflpGameRotationRaw {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AttitudeData {
    /// 欧拉角 roll，单位为弧度
    pub roll: f32,
    /// 欧拉角 pitch，单位为弧度
    pub pitch: f32,
    /// 欧拉角 yaw，单位为弧度
    pub yaw: f32,
    /// 四元数标量分量
    pub w: f32,
    /// 四元数 x 分量
    pub x: f32,
    /// 四元数 y 分量
    pub y: f32,
    /// 四元数 z 分量
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
