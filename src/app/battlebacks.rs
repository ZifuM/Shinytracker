//! Battle backgrounds and the bases Pokémon stand on (512×288, Pokémon
//! Essentials' DS-style set), built into the app.

pub(super) static FILES: &[(&str, &[u8])] = &[
    ("cave1_base0", include_bytes!("../../assets/battlebacks/cave1_base0.png")),
    ("cave1_base1", include_bytes!("../../assets/battlebacks/cave1_base1.png")),
    ("cave1_bg", include_bytes!("../../assets/battlebacks/cave1_bg.png")),
    ("cave1_ice_base0", include_bytes!("../../assets/battlebacks/cave1_ice_base0.png")),
    ("cave1_ice_base1", include_bytes!("../../assets/battlebacks/cave1_ice_base1.png")),
    ("cave1_water_base0", include_bytes!("../../assets/battlebacks/cave1_water_base0.png")),
    ("cave1_water_base1", include_bytes!("../../assets/battlebacks/cave1_water_base1.png")),
    ("cave2_bg", include_bytes!("../../assets/battlebacks/cave2_bg.png")),
    ("cave3_bg", include_bytes!("../../assets/battlebacks/cave3_bg.png")),
    ("champion1_base0", include_bytes!("../../assets/battlebacks/champion1_base0.png")),
    ("champion1_base1", include_bytes!("../../assets/battlebacks/champion1_base1.png")),
    ("champion1_bg", include_bytes!("../../assets/battlebacks/champion1_bg.png")),
    ("champion2_base0", include_bytes!("../../assets/battlebacks/champion2_base0.png")),
    ("champion2_base1", include_bytes!("../../assets/battlebacks/champion2_base1.png")),
    ("champion2_bg", include_bytes!("../../assets/battlebacks/champion2_bg.png")),
    ("city_base0", include_bytes!("../../assets/battlebacks/city_base0.png")),
    ("city_base1", include_bytes!("../../assets/battlebacks/city_base1.png")),
    ("city_bg", include_bytes!("../../assets/battlebacks/city_bg.png")),
    ("city_eve_bg", include_bytes!("../../assets/battlebacks/city_eve_bg.png")),
    ("city_night_bg", include_bytes!("../../assets/battlebacks/city_night_bg.png")),
    ("distortion_base0", include_bytes!("../../assets/battlebacks/distortion_base0.png")),
    ("distortion_base1", include_bytes!("../../assets/battlebacks/distortion_base1.png")),
    ("distortion_bg", include_bytes!("../../assets/battlebacks/distortion_bg.png")),
    ("elite1_base0", include_bytes!("../../assets/battlebacks/elite1_base0.png")),
    ("elite1_base1", include_bytes!("../../assets/battlebacks/elite1_base1.png")),
    ("elite1_bg", include_bytes!("../../assets/battlebacks/elite1_bg.png")),
    ("elite2_bg", include_bytes!("../../assets/battlebacks/elite2_bg.png")),
    ("elite3_bg", include_bytes!("../../assets/battlebacks/elite3_bg.png")),
    ("elite4_bg", include_bytes!("../../assets/battlebacks/elite4_bg.png")),
    ("field_base0", include_bytes!("../../assets/battlebacks/field_base0.png")),
    ("field_base1", include_bytes!("../../assets/battlebacks/field_base1.png")),
    ("field_bg", include_bytes!("../../assets/battlebacks/field_bg.png")),
    ("field_eve_base0", include_bytes!("../../assets/battlebacks/field_eve_base0.png")),
    ("field_eve_base1", include_bytes!("../../assets/battlebacks/field_eve_base1.png")),
    ("field_eve_bg", include_bytes!("../../assets/battlebacks/field_eve_bg.png")),
    ("field_night_base0", include_bytes!("../../assets/battlebacks/field_night_base0.png")),
    ("field_night_base1", include_bytes!("../../assets/battlebacks/field_night_base1.png")),
    ("field_night_bg", include_bytes!("../../assets/battlebacks/field_night_bg.png")),
    ("forest_base0", include_bytes!("../../assets/battlebacks/forest_base0.png")),
    ("forest_base1", include_bytes!("../../assets/battlebacks/forest_base1.png")),
    ("forest_bg", include_bytes!("../../assets/battlebacks/forest_bg.png")),
    ("forest_eve_bg", include_bytes!("../../assets/battlebacks/forest_eve_bg.png")),
    ("forest_night_bg", include_bytes!("../../assets/battlebacks/forest_night_bg.png")),
    ("grass_base0", include_bytes!("../../assets/battlebacks/grass_base0.png")),
    ("grass_base1", include_bytes!("../../assets/battlebacks/grass_base1.png")),
    ("grass_eve_base0", include_bytes!("../../assets/battlebacks/grass_eve_base0.png")),
    ("grass_eve_base1", include_bytes!("../../assets/battlebacks/grass_eve_base1.png")),
    ("grass_night_base0", include_bytes!("../../assets/battlebacks/grass_night_base0.png")),
    ("grass_night_base1", include_bytes!("../../assets/battlebacks/grass_night_base1.png")),
    ("ice_base0", include_bytes!("../../assets/battlebacks/ice_base0.png")),
    ("ice_base1", include_bytes!("../../assets/battlebacks/ice_base1.png")),
    ("indoor1_base0", include_bytes!("../../assets/battlebacks/indoor1_base0.png")),
    ("indoor1_base1", include_bytes!("../../assets/battlebacks/indoor1_base1.png")),
    ("indoor1_bg", include_bytes!("../../assets/battlebacks/indoor1_bg.png")),
    ("indoor2_bg", include_bytes!("../../assets/battlebacks/indoor2_bg.png")),
    ("indoor3_base0", include_bytes!("../../assets/battlebacks/indoor3_base0.png")),
    ("indoor3_base1", include_bytes!("../../assets/battlebacks/indoor3_base1.png")),
    ("indoor3_bg", include_bytes!("../../assets/battlebacks/indoor3_bg.png")),
    ("rocky_base0", include_bytes!("../../assets/battlebacks/rocky_base0.png")),
    ("rocky_base1", include_bytes!("../../assets/battlebacks/rocky_base1.png")),
    ("rocky_bg", include_bytes!("../../assets/battlebacks/rocky_bg.png")),
    ("rocky_eve_base0", include_bytes!("../../assets/battlebacks/rocky_eve_base0.png")),
    ("rocky_eve_base1", include_bytes!("../../assets/battlebacks/rocky_eve_base1.png")),
    ("rocky_eve_bg", include_bytes!("../../assets/battlebacks/rocky_eve_bg.png")),
    ("rocky_night_base0", include_bytes!("../../assets/battlebacks/rocky_night_base0.png")),
    ("rocky_night_base1", include_bytes!("../../assets/battlebacks/rocky_night_base1.png")),
    ("rocky_night_bg", include_bytes!("../../assets/battlebacks/rocky_night_bg.png")),
    ("sand_base0", include_bytes!("../../assets/battlebacks/sand_base0.png")),
    ("sand_base1", include_bytes!("../../assets/battlebacks/sand_base1.png")),
    ("sand_eve_base0", include_bytes!("../../assets/battlebacks/sand_eve_base0.png")),
    ("sand_eve_base1", include_bytes!("../../assets/battlebacks/sand_eve_base1.png")),
    ("sand_night_base0", include_bytes!("../../assets/battlebacks/sand_night_base0.png")),
    ("sand_night_base1", include_bytes!("../../assets/battlebacks/sand_night_base1.png")),
    ("snow_base0", include_bytes!("../../assets/battlebacks/snow_base0.png")),
    ("snow_base1", include_bytes!("../../assets/battlebacks/snow_base1.png")),
    ("snow_bg", include_bytes!("../../assets/battlebacks/snow_bg.png")),
    ("snow_eve_base0", include_bytes!("../../assets/battlebacks/snow_eve_base0.png")),
    ("snow_eve_base1", include_bytes!("../../assets/battlebacks/snow_eve_base1.png")),
    ("snow_eve_bg", include_bytes!("../../assets/battlebacks/snow_eve_bg.png")),
    ("snow_night_base0", include_bytes!("../../assets/battlebacks/snow_night_base0.png")),
    ("snow_night_base1", include_bytes!("../../assets/battlebacks/snow_night_base1.png")),
    ("snow_night_bg", include_bytes!("../../assets/battlebacks/snow_night_bg.png")),
    ("underwater_base0", include_bytes!("../../assets/battlebacks/underwater_base0.png")),
    ("underwater_base1", include_bytes!("../../assets/battlebacks/underwater_base1.png")),
    ("underwater_bg", include_bytes!("../../assets/battlebacks/underwater_bg.png")),
    ("water_base0", include_bytes!("../../assets/battlebacks/water_base0.png")),
    ("water_base1", include_bytes!("../../assets/battlebacks/water_base1.png")),
    ("water_bg", include_bytes!("../../assets/battlebacks/water_bg.png")),
    ("water_eve_base0", include_bytes!("../../assets/battlebacks/water_eve_base0.png")),
    ("water_eve_base1", include_bytes!("../../assets/battlebacks/water_eve_base1.png")),
    ("water_eve_bg", include_bytes!("../../assets/battlebacks/water_eve_bg.png")),
    ("water_night_base0", include_bytes!("../../assets/battlebacks/water_night_base0.png")),
    ("water_night_base1", include_bytes!("../../assets/battlebacks/water_night_base1.png")),
    ("water_night_bg", include_bytes!("../../assets/battlebacks/water_night_bg.png")),
];

pub(super) fn bytes(name: &str) -> Option<&'static [u8]> {
    FILES.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

/// A place to battle: the background and the two bases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Scene {
    pub label: &'static str,
    pub bg: &'static str,
    pub base: &'static str,
}

pub(super) const SCENES: [Scene; 17] = [
    Scene { label: "Tall grass", bg: "field", base: "grass" },
    Scene { label: "Field", bg: "field", base: "field" },
    Scene { label: "Forest", bg: "forest", base: "forest" },
    Scene { label: "Water", bg: "water", base: "water" },
    Scene { label: "Cave", bg: "cave1", base: "cave1" },
    Scene { label: "Cave water", bg: "cave2", base: "cave1_water" },
    Scene { label: "Ice cave", bg: "cave3", base: "cave1_ice" },
    Scene { label: "Mountain", bg: "rocky", base: "rocky" },
    Scene { label: "Sand", bg: "rocky", base: "sand" },
    Scene { label: "Snow", bg: "snow", base: "snow" },
    Scene { label: "City", bg: "city", base: "city" },
    Scene { label: "Indoors", bg: "indoor1", base: "indoor1" },
    Scene { label: "Gym", bg: "indoor3", base: "indoor3" },
    Scene { label: "Elite Four", bg: "elite1", base: "elite1" },
    Scene { label: "Champion", bg: "champion1", base: "champion1" },
    Scene { label: "Distortion World", bg: "distortion", base: "distortion" },
    Scene { label: "Underwater", bg: "underwater", base: "underwater" },
];

/// Day, evening or night.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum Light {
    #[default]
    Day,
    Evening,
    Night,
}

/// The file for a piece of a scene in this light, falling back to the day one.
pub(super) fn file(stem: &str, part: &str, light: Light) -> String {
    let lit = match light {
        Light::Day => String::new(),
        Light::Evening => "_eve".into(),
        Light::Night => "_night".into(),
    };
    let want = format!("{stem}{lit}_{part}");
    if bytes(&want).is_some() {
        want
    } else {
        format!("{stem}_{part}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_scene_has_its_pictures() {
        for s in SCENES {
            for l in [Light::Day, Light::Evening, Light::Night] {
                assert!(bytes(&file(s.bg, "bg", l)).is_some(), "{} bg", s.label);
                assert!(bytes(&file(s.base, "base0", l)).is_some(), "{} base0", s.label);
                assert!(bytes(&file(s.base, "base1", l)).is_some(), "{} base1", s.label);
            }
        }
    }
}
