//! Players' progress kept by name: their XP (and so their Battle Rank) and what they've done,
//! in a file next to the world's state (`profiles.ron`). A name is all there is to a player
//! (no accounts): whoever joins under a name takes up its progress.

use std::collections::HashMap;
use std::path::PathBuf;

use ps_sim::progress::Award;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub xp: u64,
    pub kills: u32,
    pub assists: u32,
    pub vehicle_kills: u32,
    pub revives: u32,
    pub hacks: u32,
    pub defends: u32,
    /// Stats counters by key (as an account's: see `crate::accounts`).
    #[serde(default)]
    pub stats: std::collections::BTreeMap<String, i64>,
}

impl Profile {
    pub fn add(&mut self, reason: Award, xp: u32) {
        self.xp = self.xp.saturating_add(xp as u64);
        let n = match reason {
            Award::Kill => &mut self.kills,
            Award::Assist => &mut self.assists,
            Award::VehicleKill(_) => &mut self.vehicle_kills,
            Award::Revive => &mut self.revives,
            Award::Hack => &mut self.hacks,
            Award::Defend => &mut self.defends,
            _ => return,
        };
        *n += 1;
    }
}

#[derive(Default)]
pub struct Profiles {
    map: HashMap<String, Profile>,
    path: Option<PathBuf>,
    pub dirty: bool,
}

/// Names count the same in any case, round any spaces.
fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl Profiles {
    /// The profiles kept at `path` (none there yet, or no path: none).
    pub fn load(path: Option<PathBuf>) -> Profiles {
        let map = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default();
        Profiles {
            map,
            path,
            dirty: false,
        }
    }

    pub fn get(&self, name: &str) -> Profile {
        self.map.get(&key(name)).cloned().unwrap_or_default()
    }

    /// Adds to `name`'s stats counters.
    pub fn add_stats(&mut self, name: &str, add: &[(String, i64)]) {
        self.dirty = true;
        let p = self.map.entry(key(name)).or_default();
        for (k, v) in add {
            *p.stats.entry(k.clone()).or_default() += v;
        }
    }

    /// `name`'s stats counters, by key.
    pub fn counters(&self, name: &str) -> Vec<(String, i64)> {
        self.map
            .get(&key(name))
            .map(|p| p.stats.iter().map(|(k, v)| (k.clone(), *v)).collect())
            .unwrap_or_default()
    }

    pub fn add(&mut self, name: &str, reason: Award, xp: u32) -> &Profile {
        self.dirty = true;
        let p = self.map.entry(key(name)).or_default();
        p.add(reason, xp);
        p
    }

    pub fn save(&mut self) {
        self.dirty = false;
        let Some(path) = &self.path else { return };
        let Ok(text) = ron::ser::to_string_pretty(&self.map, Default::default()) else {
            return;
        };
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_add_up_and_round_trip() {
        let dir = std::env::temp_dir().join(format!("ps-profiles-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("profiles.ron");
        let mut p = Profiles::load(Some(path.clone()));
        p.add("Ace", Award::Kill, 100);
        p.add("ace ", Award::Headshot, 25);
        p.add("Ace", Award::Revive, 75);
        assert_eq!(p.get("ACE").xp, 200);
        assert_eq!(p.get("Ace").kills, 1);
        assert_eq!(p.get("Ace").revives, 1);
        p.save();
        let back = Profiles::load(Some(path));
        assert_eq!(back.get("ace"), p.get("ace"));
        assert_eq!(back.get("nobody"), Profile::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
