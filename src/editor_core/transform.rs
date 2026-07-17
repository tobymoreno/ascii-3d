#[derive(Clone, Debug, PartialEq)]
pub enum EditorTransformCommand<T> {
    Translate { target: T, delta: [f32; 3] },
    Rotate { target: T, delta_degrees: [f32; 3] },
    ScaleUniform { target: T, factor: f32 },
    Reset { target: T },
}

impl<T> EditorTransformCommand<T> {
    pub fn target(&self) -> &T {
        match self {
            Self::Translate { target, .. }
            | Self::Rotate { target, .. }
            | Self::ScaleUniform { target, .. }
            | Self::Reset { target } => target,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::Translate { delta, .. }
            | Self::Rotate {
                delta_degrees: delta,
                ..
            } => delta.iter().all(|component| component.is_finite()),
            Self::ScaleUniform { factor, .. } => factor.is_finite() && *factor > 0.0,
            Self::Reset { .. } => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EditorTransformCommand;

    #[test]
    fn rejects_non_finite_transform_values() {
        assert!(
            !EditorTransformCommand::Translate {
                target: "earth",
                delta: [f32::NAN, 0.0, 0.0],
            }
            .is_valid()
        );
        assert!(
            !EditorTransformCommand::ScaleUniform {
                target: "earth",
                factor: 0.0,
            }
            .is_valid()
        );
    }
}
