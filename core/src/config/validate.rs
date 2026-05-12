use crate::config::types::{ConfigError, DeviceConfig};

pub fn validate_config(config: &DeviceConfig) -> Result<(), ConfigError> {
    if !(config.report.report_hz.is_finite() && config.report.report_hz > 0.0) {
        return Err(ConfigError::ValidationFailed);
    }

    let motion = config.motion;
    for value in [
        motion.deadzone_x_rad,
        motion.deadzone_y_rad,
        motion.deadzone_speed,
        motion.max_angle_rad,
        motion.sensitivity_x,
        motion.sensitivity_y,
        motion.smoothing_alpha,
    ] {
        if !value.is_finite() {
            return Err(ConfigError::ValidationFailed);
        }
    }

    if motion.deadzone_x_rad < 0.0
        || motion.deadzone_y_rad < 0.0
        || motion.deadzone_speed < 0.0
        || motion.max_angle_rad <= 0.0
        || motion.sensitivity_x <= 0.0
        || motion.sensitivity_y <= 0.0
        || !(0.0..=1.0).contains(&motion.smoothing_alpha)
    {
        return Err(ConfigError::ValidationFailed);
    }

    let power = config.power;
    if power.suspend_timeout_ms == 0 || power.disconnect_sleep_timeout_ms == 0 {
        return Err(ConfigError::ValidationFailed);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> DeviceConfig {
        DeviceConfig::default()
    }

    #[test]
    fn default_is_ok() {
        assert!(validate_config(&valid()).is_ok());
    }

    #[test]
    fn reject_zero_report_hz() {
        let mut c = valid();
        c.report.report_hz = 0.0;
        assert_eq!(validate_config(&c), Err(ConfigError::ValidationFailed));
    }

    #[test]
    fn reject_nan_deadzone() {
        let mut c = valid();
        c.motion.deadzone_x_rad = f32::NAN;
        assert_eq!(validate_config(&c), Err(ConfigError::ValidationFailed));
    }

    #[test]
    fn reject_zero_power_timeout() {
        let mut c = valid();
        c.power.suspend_timeout_ms = 0;
        assert_eq!(validate_config(&c), Err(ConfigError::ValidationFailed));
    }
}
