use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    SessionStarted { game_id: i64 },
    SessionEnded { game_id: i64, session_id: i64 },
    FocusStarted { game_id: i64, session_id: i64 },
    FocusEnded { game_id: i64 },
}
#[derive(Default)]
pub struct TrackerState {
    running: HashMap<i64, i64>,
    focused: Option<i64>,
}
impl TrackerState {
    pub fn reconcile_running<F>(&mut self, alive: &HashSet<i64>, mut create: F) -> Vec<Action>
    where
        F: FnMut(i64) -> i64,
    {
        let mut a = vec![];
        let ended: Vec<_> = self
            .running
            .keys()
            .filter(|g| !alive.contains(g))
            .copied()
            .collect();
        for g in ended {
            if self.focused == Some(g) {
                self.focused = None;
                a.push(Action::FocusEnded { game_id: g })
            }
            let s = self.running.remove(&g).unwrap();
            a.push(Action::SessionEnded {
                game_id: g,
                session_id: s,
            })
        }
        for &g in alive {
            if let std::collections::hash_map::Entry::Vacant(entry) = self.running.entry(g) {
                let s = create(g);
                entry.insert(s);
                a.push(Action::SessionStarted { game_id: g })
            }
        }
        a
    }
    pub fn foreground(&mut self, game: Option<i64>) -> Vec<Action> {
        let game = game.filter(|g| self.running.contains_key(g));
        if game == self.focused {
            return vec![];
        }
        let mut a = vec![];
        if let Some(g) = self.focused.take() {
            a.push(Action::FocusEnded { game_id: g })
        }
        if let Some(g) = game {
            self.focused = Some(g);
            a.push(Action::FocusStarted {
                game_id: g,
                session_id: self.running[&g],
            })
        }
        a
    }
    pub fn running(&self) -> &HashMap<i64, i64> {
        &self.running
    }
    pub fn focused(&self) -> Option<i64> {
        self.focused
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn set(xs: &[i64]) -> HashSet<i64> {
        xs.iter().copied().collect()
    }
    #[test]
    fn normal_flow() {
        let mut s = TrackerState::default();
        assert_eq!(
            s.reconcile_running(&set(&[1]), |_| 10),
            vec![Action::SessionStarted { game_id: 1 }]
        );
        assert!(matches!(
            s.foreground(Some(1))[0],
            Action::FocusStarted { .. }
        ));
        assert!(matches!(s.foreground(None)[0], Action::FocusEnded { .. }));
        assert!(matches!(
            s.foreground(Some(1))[0],
            Action::FocusStarted { .. }
        ));
        let a = s.reconcile_running(&set(&[]), |_| 0);
        assert!(
            a.iter()
                .any(|x| matches!(x, Action::SessionEnded { session_id: 10, .. }))
        );
    }
    #[test]
    fn launcher_does_not_duplicate() {
        let mut s = TrackerState::default();
        s.reconcile_running(&set(&[1]), |_| 10);
        assert!(s.reconcile_running(&set(&[1]), |_| 11).is_empty());
        assert_eq!(s.running()[&1], 10);
    }
    #[test]
    fn multiple_games_focus() {
        let mut s = TrackerState::default();
        s.reconcile_running(&set(&[1, 2]), |g| g + 10);
        assert_eq!(s.foreground(Some(1)).len(), 1);
        let x = s.foreground(Some(2));
        assert_eq!(x.len(), 2);
        assert!(matches!(x[0], Action::FocusEnded { game_id: 1 }));
        assert!(matches!(x[1], Action::FocusStarted { game_id: 2, .. }));
        assert_eq!(s.foreground(None).len(), 1);
        assert_eq!(s.foreground(Some(1)).len(), 1);
    }
}
