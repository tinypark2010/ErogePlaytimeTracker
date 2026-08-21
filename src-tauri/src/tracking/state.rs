use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    SessionStarted { game_id: i64 },
    SessionEnded { game_id: i64, session_id: i64 },
    FocusStarted { game_id: i64, session_id: i64 },
    FocusEnded { game_id: i64, session_id: i64 },
    BackgroundStarted { game_id: i64, session_id: i64 },
    BackgroundEnded { game_id: i64, session_id: i64 },
}

#[derive(Default)]
pub struct TrackerState {
    running: HashMap<i64, i64>,
    focused: Option<i64>,
    background: HashSet<i64>,
}

impl TrackerState {
    pub fn reconcile_running<F>(&mut self, alive: &HashSet<i64>, mut create: F) -> Vec<Action>
    where
        F: FnMut(i64) -> i64,
    {
        let mut actions = vec![];
        let ended: Vec<_> = self
            .running
            .keys()
            .filter(|game| !alive.contains(game))
            .copied()
            .collect();
        for game in ended {
            let session = self.running[&game];
            if self.focused == Some(game) {
                self.focused = None;
                actions.push(Action::FocusEnded {
                    game_id: game,
                    session_id: session,
                });
            }
            if self.background.remove(&game) {
                actions.push(Action::BackgroundEnded {
                    game_id: game,
                    session_id: session,
                });
            }
            self.running.remove(&game);
            actions.push(Action::SessionEnded {
                game_id: game,
                session_id: session,
            });
        }
        for &game in alive {
            if let std::collections::hash_map::Entry::Vacant(entry) = self.running.entry(game) {
                entry.insert(create(game));
                actions.push(Action::SessionStarted { game_id: game });
            }
        }
        actions
    }

    pub fn observe_windows(
        &mut self,
        foreground: Option<i64>,
        windowed: &HashSet<i64>,
    ) -> Vec<Action> {
        let foreground =
            foreground.filter(|game| self.running.contains_key(game) && windowed.contains(game));
        let mut actions = vec![];
        if foreground != self.focused {
            if let Some(game) = self.focused.take() {
                actions.push(Action::FocusEnded {
                    game_id: game,
                    session_id: self.running[&game],
                });
            }
            if let Some(game) = foreground {
                self.focused = Some(game);
                actions.push(Action::FocusStarted {
                    game_id: game,
                    session_id: self.running[&game],
                });
            }
        }

        let desired_background: HashSet<_> = self
            .running
            .keys()
            .filter(|game| windowed.contains(game) && foreground != Some(**game))
            .copied()
            .collect();
        let ended: Vec<_> = self
            .background
            .difference(&desired_background)
            .copied()
            .collect();
        for game in ended {
            self.background.remove(&game);
            actions.push(Action::BackgroundEnded {
                game_id: game,
                session_id: self.running[&game],
            });
        }
        let started: Vec<_> = desired_background
            .difference(&self.background)
            .copied()
            .collect();
        for game in started {
            self.background.insert(game);
            actions.push(Action::BackgroundStarted {
                game_id: game,
                session_id: self.running[&game],
            });
        }
        actions
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
    fn set(values: &[i64]) -> HashSet<i64> {
        values.iter().copied().collect()
    }

    #[test]
    fn startup_without_a_window_is_not_background() {
        let mut state = TrackerState::default();
        state.reconcile_running(&set(&[1]), |_| 10);
        assert!(state.observe_windows(None, &set(&[])).is_empty());
        assert!(matches!(
            state.observe_windows(Some(1), &set(&[1]))[0],
            Action::FocusStarted { .. }
        ));
    }

    #[test]
    fn visible_non_foreground_window_is_background() {
        let mut state = TrackerState::default();
        state.reconcile_running(&set(&[1]), |_| 10);
        assert!(matches!(
            state.observe_windows(None, &set(&[1]))[0],
            Action::BackgroundStarted { .. }
        ));
        assert_eq!(state.observe_windows(Some(1), &set(&[1])).len(), 2);
    }

    #[test]
    fn destroyed_window_does_not_start_background() {
        let mut state = TrackerState::default();
        state.reconcile_running(&set(&[1]), |_| 10);
        state.observe_windows(Some(1), &set(&[1]));
        let actions = state.observe_windows(None, &set(&[]));
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::FocusEnded { .. }));
    }

    #[test]
    fn launcher_does_not_duplicate_session() {
        let mut state = TrackerState::default();
        state.reconcile_running(&set(&[1]), |_| 10);
        assert!(state.reconcile_running(&set(&[1]), |_| 11).is_empty());
        assert_eq!(state.running()[&1], 10);
    }

    #[test]
    fn multiple_games_track_background_independently() {
        let mut state = TrackerState::default();
        state.reconcile_running(&set(&[1, 2]), |game| game + 10);
        let actions = state.observe_windows(Some(1), &set(&[1, 2]));
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::BackgroundStarted { game_id: 2, .. }))
        );
        let actions = state.observe_windows(Some(2), &set(&[1, 2]));
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::BackgroundStarted { game_id: 1, .. }))
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::BackgroundEnded { game_id: 2, .. }))
        );
    }
}
