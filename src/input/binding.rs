use super::AppCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBinding {
    pub key: &'static str,
    pub label: &'static str,
    pub command: AppCommand,
}
