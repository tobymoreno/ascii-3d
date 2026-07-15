use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

const INITIAL_REPEAT_DELAY: Duration = Duration::from_millis(300);
const REPEAT_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Debug, Default)]
pub struct EditorKeyRepeatGate {
    active_key: Option<KeyCode>,
    next_allowed: Option<Instant>,
}

impl EditorKeyRepeatGate {
    pub fn accept(&mut self, key: KeyEvent, now: Instant) -> bool {
        match key.kind {
            KeyEventKind::Release => {
                if self.active_key == Some(key.code) {
                    self.reset();
                }
                false
            }
            KeyEventKind::Press | KeyEventKind::Repeat => {
                if !is_navigation_key(key.code) {
                    return key.kind == KeyEventKind::Press;
                }

                if self.active_key != Some(key.code) {
                    self.active_key = Some(key.code);
                    self.next_allowed = Some(now + INITIAL_REPEAT_DELAY);
                    return true;
                }

                let Some(next_allowed) = self.next_allowed else {
                    self.next_allowed = Some(now + INITIAL_REPEAT_DELAY);
                    return true;
                };

                if now < next_allowed {
                    return false;
                }

                self.next_allowed = Some(now + REPEAT_INTERVAL);
                true
            }
        }
    }

    pub fn reset(&mut self) {
        self.active_key = None;
        self.next_allowed = None;
    }
}

fn is_navigation_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Char('j')
            | KeyCode::Char('J')
            | KeyCode::Char('k')
            | KeyCode::Char('K')
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
        KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind)
    }

    #[test]
    fn first_navigation_press_is_immediate() {
        let mut gate = EditorKeyRepeatGate::default();
        let now = Instant::now();
        assert!(gate.accept(key(KeyCode::Down, KeyEventKind::Press), now));
    }

    #[test]
    fn burst_press_events_are_coalesced() {
        let mut gate = EditorKeyRepeatGate::default();
        let now = Instant::now();
        assert!(gate.accept(key(KeyCode::Down, KeyEventKind::Press), now));
        assert!(!gate.accept(
            key(KeyCode::Down, KeyEventKind::Press),
            now + Duration::from_millis(30)
        ));
    }

    #[test]
    fn held_navigation_repeats_at_controlled_rate() {
        let mut gate = EditorKeyRepeatGate::default();
        let now = Instant::now();
        assert!(gate.accept(key(KeyCode::Down, KeyEventKind::Press), now));
        assert!(!gate.accept(
            key(KeyCode::Down, KeyEventKind::Repeat),
            now + Duration::from_millis(299)
        ));
        assert!(gate.accept(
            key(KeyCode::Down, KeyEventKind::Repeat),
            now + Duration::from_millis(300)
        ));
        assert!(!gate.accept(
            key(KeyCode::Down, KeyEventKind::Repeat),
            now + Duration::from_millis(350)
        ));
        assert!(gate.accept(
            key(KeyCode::Down, KeyEventKind::Repeat),
            now + Duration::from_millis(420)
        ));
    }

    #[test]
    fn release_allows_immediate_repress() {
        let mut gate = EditorKeyRepeatGate::default();
        let now = Instant::now();
        assert!(gate.accept(key(KeyCode::Down, KeyEventKind::Press), now));
        assert!(!gate.accept(
            key(KeyCode::Down, KeyEventKind::Release),
            now + Duration::from_millis(20)
        ));
        assert!(gate.accept(
            key(KeyCode::Down, KeyEventKind::Press),
            now + Duration::from_millis(40)
        ));
    }
}
