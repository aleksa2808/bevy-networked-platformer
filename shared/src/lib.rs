#![feature(iter_zip)]

pub mod game;

pub use bevy;
use crystalorb::Config;
pub use crystalorb_bevy_networking_turbulence;
pub use game::PlayerId;
use serde::{Deserialize, Serialize};

pub const SERVER_PORT: u16 = 1212;
pub const TIMESTEP: f64 = 1.0 / 60.0;

pub fn crystal_orb_config() -> Config {
    Config {
        //lag_compensation_latency: (),
        //blend_latency: 0.001,
        timestep_seconds: TIMESTEP,
        //clock_sync_needed_sample_count: (),
        //clock_sync_assumed_outlier_rate: (),
        //clock_sync_request_period: (),
        //max_tolerable_clock_deviation: (),
        //snapshot_send_period: (),
        //update_delta_seconds_max: (),
        //timestamp_skip_threshold_seconds: (),
        //fastforward_max_per_step: (),
        //tweening_method: (),
        ..Default::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Position {
    pub y: f32,
    pub x: f32,
}

pub const BOTTOM_START_POSITION: Position = Position { y: 400.0, x: 150.0 };
pub const TOP_START_POSITION: Position = Position { y: 600.0, x: 850.0 };

pub struct Rect {
    pub y: f32,
    pub x: f32,
    pub h: f32,
    pub w: f32,
}

pub const PLATFORMS: [Rect; 12] = [
    // left power platform
    Rect {
        y: 250.0,
        x: 150.0,
        h: 100.0,
        w: 100.0,
    },
    // bottom platform
    Rect {
        y: 150.0,
        x: 500.0,
        h: 100.0,
        w: 800.0,
    },
    // right power platform
    Rect {
        y: 250.0,
        x: 850.0,
        h: 100.0,
        w: 100.0,
    },
    // middle platforms
    Rect {
        y: 270.0,
        x: 250.0,
        h: 20.0,
        w: 40.0,
    },
    Rect {
        y: 230.0,
        x: 320.0,
        h: 60.0,
        w: 20.0,
    },
    Rect {
        y: 250.0,
        x: 400.0,
        h: 20.0,
        w: 60.0,
    },
    // 4 squares
    Rect {
        y: 260.0,
        x: 470.0,
        h: 20.0,
        w: 20.0,
    },
    Rect {
        y: 250.0,
        x: 515.0,
        h: 20.0,
        w: 20.0,
    },
    Rect {
        y: 270.0,
        x: 560.0,
        h: 20.0,
        w: 20.0,
    },
    Rect {
        y: 240.0,
        x: 605.0,
        h: 20.0,
        w: 20.0,
    },
    // platform touching lava
    Rect {
        y: 220.0,
        x: 680.0,
        h: 20.0,
        w: 80.0,
    },
    Rect {
        y: 260.0,
        x: 760.0,
        h: 20.0,
        w: 20.0,
    },
];

pub const LAVA_RECTS: [Rect; 2] = [
    // left power platform
    Rect {
        y: 210.0,
        x: 500.0,
        h: 20.0,
        w: 600.0,
    },
    // bottom platform
    Rect {
        y: 20.0,
        x: 500.0,
        h: 40.0,
        w: 2000.0,
    },
];

pub struct PowerPadPositions {
    pub left: Position,
    pub right: Position,
}

pub const BOTTOM_POWER_PAD_POSITIONS: PowerPadPositions = PowerPadPositions {
    left: Position { x: 150.0, y: 295.0 },
    right: Position { x: 850.0, y: 295.0 },
};
pub const STARTING_BOTTOM_POWER_PAD_POSITION: Position = BOTTOM_POWER_PAD_POSITIONS.right;

pub const TOP_POWER_PAD_POSITIONS: PowerPadPositions = PowerPadPositions {
    left: Position { x: 150.0, y: 705.0 },
    right: Position { x: 850.0, y: 705.0 },
};
pub const STARTING_TOP_POWER_PAD_POSITION: Position = TOP_POWER_PAD_POSITIONS.left;

pub struct Size {
    pub w: f32,
    pub h: f32,
}

pub const POWER_PAD_SIZE: Size = Size { w: 70.0, h: 10.0 };
pub const PROJECTILE_SIZE: Size = Size { w: 10.0, h: 40.0 };
