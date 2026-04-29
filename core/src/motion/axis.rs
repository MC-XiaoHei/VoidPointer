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
    pub fn extract(&self, attitude: &crate::attitude::types::AttitudeData) -> f32 {
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
