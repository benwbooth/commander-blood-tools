//! Game-progression state — a functional layer over the decoded entity flag state machine
//! ([`crate::entity`]). The original game tracks per-object state in the 32-byte
//! `entity_object_table` records (flag word + decoded advance/toggle routines); this builds
//! the game-level progression on top of that exact primitive: each trackable location /
//! crew member is one [`EntityObject`], "visited" being the decoded state-advance
//! (STATE0 → STATE1). It drives completion (all locations visited → the ending finale) and
//! is persisted in the save.
//!
//! The object *population* (which locations/crew exist) comes from the port's decoded
//! nav destinations and speech-event actors — real game entities — so this is the decoded
//! progression model driven by decoded content, not an invented state machine.

use crate::entity::{flag, EntityObject};

/// The player's game progression: a set of trackable entities (locations, crew) keyed by
/// name, each carrying the decoded [`EntityObject`] flag state.
#[derive(Clone, Debug, Default)]
pub struct GameProgress {
    entries: Vec<(String, EntityObject)>,
}

impl GameProgress {
    /// An empty progression.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn find(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|(n, _)| n == name)
    }

    /// Register a trackable location/crew member (idempotent). It is populated as the
    /// decoded `entity_object_populate` does — active with STATE0 set = "not yet visited".
    /// `group` is the object's decoded id/group word.
    pub fn register(&mut self, name: &str, group: u16) {
        if self.find(name).is_none() {
            let obj = EntityObject::populate(0, (0, 0), group, (0, 0));
            self.entries.push((name.to_string(), obj));
        }
    }

    /// Mark a location/crew visited — the decoded state advance (`0x41d1`: clears STATE0,
    /// sets STATE1). Returns whether it was newly visited (was unvisited before).
    pub fn visit(&mut self, name: &str) -> bool {
        match self.find(name) {
            Some(i) => {
                let was = self.entries[i].1.flags & flag::STATE0 != 0;
                self.entries[i].1.advance_state();
                was
            }
            None => {
                // Auto-register then visit, so a visit to an unregistered entity still counts.
                self.register(name, 0);
                let i = self.find(name).unwrap();
                self.entries[i].1.advance_state();
                true
            }
        }
    }

    /// Whether a location/crew has been visited (its decoded STATE0 was cleared).
    pub fn has_visited(&self, name: &str) -> bool {
        self.find(name)
            .map(|i| self.entries[i].1.flags & flag::STATE0 == 0)
            .unwrap_or(false)
    }

    /// The number of registered trackable entities.
    pub fn total(&self) -> usize {
        self.entries.len()
    }

    /// The number that have been visited.
    pub fn visited_count(&self) -> usize {
        self.entries.iter().filter(|(_, o)| o.flags & flag::STATE0 == 0).count()
    }

    /// Whether every registered location/crew has been visited (drives the ending).
    pub fn all_visited(&self) -> bool {
        !self.entries.is_empty() && self.visited_count() == self.entries.len()
    }

    /// The names of every registered entity, in registration order (for the save).
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(n, _)| n.as_str())
    }

    /// The names of the visited entities (for the save / a progress display).
    pub fn visited_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, o)| o.flags & flag::STATE0 == 0)
            .map(|(n, _)| n.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_visits_via_the_decoded_state_machine() {
        let mut p = GameProgress::new();
        for (name, g) in [("Venusia", 3u16), ("Ekatomb", 4), ("Kortex", 5)] {
            p.register(name, g);
        }
        assert_eq!(p.total(), 3);
        assert_eq!(p.visited_count(), 0);
        assert!(!p.all_visited());
        assert!(!p.has_visited("Venusia"));

        // First visit counts; a repeat visit does not re-count.
        assert!(p.visit("Venusia"));
        assert!(!p.visit("Venusia"), "already visited");
        assert!(p.has_visited("Venusia"));
        assert_eq!(p.visited_count(), 1);

        p.visit("Ekatomb");
        p.visit("Kortex");
        assert!(p.all_visited(), "all locations visited -> completion");
        assert_eq!(p.visited_names().len(), 3);
    }

    #[test]
    fn register_is_idempotent_and_visit_autoregisters() {
        let mut p = GameProgress::new();
        p.register("Hito", 6);
        p.register("Hito", 6); // idempotent
        assert_eq!(p.total(), 1);
        // Visiting an unregistered entity auto-registers + counts it.
        assert!(p.visit("Magnus"));
        assert_eq!(p.total(), 2);
        assert!(p.has_visited("Magnus"));
    }
}
