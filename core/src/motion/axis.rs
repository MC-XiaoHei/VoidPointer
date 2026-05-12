use crate::attitude::types::AttitudeData;

pub const HW_MAP_X: AxisMap = AxisMap {
    source: SourceAxis::Yaw,
    dir: AxisDir::Inverted,
};
pub const HW_MAP_Y: AxisMap = AxisMap {
    source: SourceAxis::Pitch,
    dir: AxisDir::Normal,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAxis {
    Roll,
    Pitch,
    Yaw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisDir {
    Normal,
    Inverted,
}

#[derive(Debug, Clone, Copy)]
pub struct AxisMap {
    pub source: SourceAxis,
    pub dir: AxisDir,
}

impl AxisMap {
    #[inline(always)]
    pub fn extract(&self, attitude: &AttitudeData) -> f32 {
        let raw_val = match self.source {
            SourceAxis::Roll => attitude.roll,
            SourceAxis::Pitch => attitude.pitch,
            SourceAxis::Yaw => attitude.yaw,
        };

        match self.dir {
            AxisDir::Normal => raw_val,
            AxisDir::Inverted => -raw_val,
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    fn attitude() -> AttitudeData {
        AttitudeData {
            roll: 1.0,
            pitch: 2.0,
            yaw: 3.0,
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[test]
    fn extract_roll_normal() {
        let map = AxisMap {
            source: SourceAxis::Roll,
            dir: AxisDir::Normal,
        };
        assert_eq!(map.extract(&attitude()), 1.0);
    }

    #[test]
    fn extract_roll_inverted() {
        let map = AxisMap {
            source: SourceAxis::Roll,
            dir: AxisDir::Inverted,
        };
        assert_eq!(map.extract(&attitude()), -1.0);
    }

    #[test]
    fn extract_pitch_normal() {
        let map = AxisMap {
            source: SourceAxis::Pitch,
            dir: AxisDir::Normal,
        };
        assert_eq!(map.extract(&attitude()), 2.0);
    }

    #[test]
    fn extract_pitch_inverted() {
        let map = AxisMap {
            source: SourceAxis::Pitch,
            dir: AxisDir::Inverted,
        };
        assert_eq!(map.extract(&attitude()), -2.0);
    }

    #[test]
    fn extract_yaw_normal() {
        let map = AxisMap {
            source: SourceAxis::Yaw,
            dir: AxisDir::Normal,
        };
        assert_eq!(map.extract(&attitude()), 3.0);
    }

    #[test]
    fn extract_yaw_inverted() {
        let map = AxisMap {
            source: SourceAxis::Yaw,
            dir: AxisDir::Inverted,
        };
        assert_eq!(map.extract(&attitude()), -3.0);
    }

    #[test]
    fn hw_map_x_is_yaw_inverted() {
        assert_eq!(HW_MAP_X.source, SourceAxis::Yaw);
        assert_eq!(HW_MAP_X.dir, AxisDir::Inverted);
    }

    #[test]
    fn hw_map_y_is_pitch_normal() {
        assert_eq!(HW_MAP_Y.source, SourceAxis::Pitch);
        assert_eq!(HW_MAP_Y.dir, AxisDir::Normal);
    }
}
