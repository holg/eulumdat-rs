//! Room type presets, reflectance presets, and LLF presets.

use super::compute::{LightLossFactor, Reflectances, Room};

/// Preset room types with standard parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomPreset {
    OpenOffice,
    PrivateOffice,
    Classroom,
    Corridor,
    Warehouse,
    Retail,
    Workshop,
    ConferenceRoom,
    Restroom,
    ParkingGarage,
}

impl RoomPreset {
    pub fn all() -> &'static [RoomPreset] {
        &[
            Self::OpenOffice,
            Self::PrivateOffice,
            Self::Classroom,
            Self::Corridor,
            Self::Warehouse,
            Self::Retail,
            Self::Workshop,
            Self::ConferenceRoom,
            Self::Restroom,
            Self::ParkingGarage,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::OpenOffice => "Open Office",
            Self::PrivateOffice => "Private Office",
            Self::Classroom => "Classroom",
            Self::Corridor => "Corridor",
            Self::Warehouse => "Warehouse",
            Self::Retail => "Retail",
            Self::Workshop => "Workshop",
            Self::ConferenceRoom => "Conference Room",
            Self::Restroom => "Restroom",
            Self::ParkingGarage => "Parking Garage",
        }
    }

    /// Target illuminance in lux (EN 12464-1 / IES RP-1).
    pub fn target_lux(&self) -> f64 {
        match self {
            Self::OpenOffice => 500.0,
            Self::PrivateOffice => 500.0,
            Self::Classroom => 500.0,
            Self::Corridor => 100.0,
            Self::Warehouse => 200.0,
            Self::Retail => 500.0,
            Self::Workshop => 750.0,
            Self::ConferenceRoom => 500.0,
            Self::Restroom => 200.0,
            Self::ParkingGarage => 75.0,
        }
    }

    pub fn to_room(&self) -> Room {
        match self {
            Self::OpenOffice => Room::new(20.0, 15.0, 3.0, 0.80, 0.0),
            Self::PrivateOffice => Room::new(4.0, 3.0, 2.8, 0.80, 0.0),
            Self::Classroom => Room::new(10.0, 8.0, 3.0, 0.80, 0.0),
            Self::Corridor => Room::new(30.0, 2.0, 2.8, 0.0, 0.0),
            Self::Warehouse => Room::new(40.0, 30.0, 6.0, 0.0, 0.0),
            Self::Retail => Room::new(20.0, 15.0, 3.5, 0.0, 0.0),
            Self::Workshop => Room::new(15.0, 10.0, 4.0, 0.85, 0.0),
            Self::ConferenceRoom => Room::new(8.0, 5.0, 2.8, 0.80, 0.0),
            Self::Restroom => Room::new(4.0, 3.0, 2.8, 0.0, 0.0),
            Self::ParkingGarage => Room::new(30.0, 15.0, 3.0, 0.0, 0.0),
        }
    }
}

/// Reflectance presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectancePreset {
    /// IES default: light ceiling, colored walls
    Standard,
    /// High-reflectance surfaces
    BrightRoom,
    /// Low-reflectance surfaces
    DarkRoom,
    /// Typical industrial
    Industrial,
}

impl ReflectancePreset {
    pub fn all() -> &'static [ReflectancePreset] {
        &[
            Self::Standard,
            Self::BrightRoom,
            Self::DarkRoom,
            Self::Industrial,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Standard => "Standard (LCC)",
            Self::BrightRoom => "Bright Room",
            Self::DarkRoom => "Dark Room",
            Self::Industrial => "Industrial",
        }
    }

    pub fn to_reflectances(&self) -> Reflectances {
        match self {
            Self::Standard => Reflectances::new(0.70, 0.50, 0.20),
            Self::BrightRoom => Reflectances::new(0.80, 0.70, 0.30),
            Self::DarkRoom => Reflectances::new(0.50, 0.30, 0.10),
            Self::Industrial => Reflectances::new(0.50, 0.30, 0.20),
        }
    }
}

/// Light Loss Factor presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlfPreset {
    Led,
    Fluorescent,
}

impl LlfPreset {
    pub fn all() -> &'static [LlfPreset] {
        &[Self::Led, Self::Fluorescent]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Led => "LED",
            Self::Fluorescent => "Fluorescent",
        }
    }

    pub fn to_llf(&self) -> LightLossFactor {
        match self {
            Self::Led => LightLossFactor::new(0.90, 0.95, 1.0, 0.98),
            Self::Fluorescent => LightLossFactor::new(0.85, 0.90, 0.95, 0.96),
        }
    }
}
