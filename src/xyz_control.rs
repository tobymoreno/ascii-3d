use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzAxis {
    X,
    Y,
    Z,
}

impl XyzAxis {
    pub const fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzDirection {
    Positive,
    Negative,
}

impl XyzDirection {
    pub const fn sign(self) -> f32 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Positive => "+",
            Self::Negative => "-",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzControlEvent {
    Rotate {
        axis: XyzAxis,
        direction: XyzDirection,
    },
    MoveOrigin {
        axis: XyzAxis,
        direction: XyzDirection,
    },
    Reset,
}

impl XyzControlEvent {
    pub fn label(self) -> String {
        match self {
            Self::Rotate { axis, direction } => {
                format!("rotate {}{}", direction.label(), axis.label())
            }
            Self::MoveOrigin { axis, direction } => {
                format!("move origin {}{}", direction.label(), axis.label())
            }
            Self::Reset => "reset".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct XyzControl {
    pub rotation_step_degrees: f32,
    pub origin_step: f32,
}

impl Default for XyzControl {
    fn default() -> Self {
        Self {
            rotation_step_degrees: 5.0,
            origin_step: 0.25,
        }
    }
}

impl XyzControl {
    pub fn event_for_key(self, key: KeyEvent) -> Option<XyzControlEvent> {
        match key.code {
            KeyCode::Left => Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::X,
                direction: XyzDirection::Negative,
            }),
            KeyCode::Right => Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::X,
                direction: XyzDirection::Positive,
            }),
            KeyCode::Up => Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::Y,
                direction: XyzDirection::Positive,
            }),
            KeyCode::Down => Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::Y,
                direction: XyzDirection::Negative,
            }),
            KeyCode::PageUp => Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::Z,
                direction: XyzDirection::Positive,
            }),
            KeyCode::PageDown => Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::Z,
                direction: XyzDirection::Negative,
            }),
            KeyCode::Char('x') => Some(XyzControlEvent::Rotate {
                axis: XyzAxis::X,
                direction: XyzDirection::Positive,
            }),
            KeyCode::Char('X') => Some(XyzControlEvent::Rotate {
                axis: XyzAxis::X,
                direction: XyzDirection::Negative,
            }),
            KeyCode::Char('y') => Some(XyzControlEvent::Rotate {
                axis: XyzAxis::Y,
                direction: XyzDirection::Positive,
            }),
            KeyCode::Char('Y') => Some(XyzControlEvent::Rotate {
                axis: XyzAxis::Y,
                direction: XyzDirection::Negative,
            }),
            KeyCode::Char('z') => Some(XyzControlEvent::Rotate {
                axis: XyzAxis::Z,
                direction: XyzDirection::Positive,
            }),
            KeyCode::Char('Z') => Some(XyzControlEvent::Rotate {
                axis: XyzAxis::Z,
                direction: XyzDirection::Negative,
            }),
            _ => None,
        }
    }

    pub fn rotation_delta(self, axis: XyzAxis, direction: XyzDirection) -> crate::math::Vec3 {
        let amount = self.rotation_step_degrees * direction.sign();

        match axis {
            XyzAxis::X => crate::math::Vec3::new(amount, 0.0, 0.0),
            XyzAxis::Y => crate::math::Vec3::new(0.0, amount, 0.0),
            XyzAxis::Z => crate::math::Vec3::new(0.0, 0.0, amount),
        }
    }

    pub fn origin_delta(self, axis: XyzAxis, direction: XyzDirection) -> crate::math::Vec3 {
        let amount = self.origin_step * direction.sign();

        match axis {
            XyzAxis::X => crate::math::Vec3::new(amount, 0.0, 0.0),
            XyzAxis::Y => crate::math::Vec3::new(0.0, amount, 0.0),
            XyzAxis::Z => crate::math::Vec3::new(0.0, 0.0, amount),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    #[test]
    fn plain_arrows_translate_active_target() {
        let control = XyzControl::default();

        assert_eq!(
            control.event_for_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::X,
                direction: XyzDirection::Negative,
            })
        );
        assert_eq!(
            control.event_for_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::Y,
                direction: XyzDirection::Positive,
            })
        );
    }

    #[test]
    fn page_keys_translate_z() {
        let control = XyzControl::default();

        assert_eq!(
            control.event_for_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
            Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::Z,
                direction: XyzDirection::Positive,
            })
        );
        assert_eq!(
            control.event_for_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            Some(XyzControlEvent::MoveOrigin {
                axis: XyzAxis::Z,
                direction: XyzDirection::Negative,
            })
        );
    }
}
