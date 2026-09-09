use accesskit::{Action, Role};

/// Semantic accessibility metadata attached to an MTK layout node.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessibleInfo {
    pub role: Role,
    pub label: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub disabled: bool,
    pub hidden: bool,
    pub toggled: Option<bool>,
    pub numeric_value: Option<f64>,
    pub min_numeric_value: Option<f64>,
    pub max_numeric_value: Option<f64>,
    pub step: Option<f64>,
    pub actions: Vec<Action>,
}

impl AccessibleInfo {
    /// Creates a new accessibility descriptor with the given semantic role.
    pub fn new(role: Role) -> Self {
        Self {
            role,
            label: None,
            value: None,
            description: None,
            disabled: false,
            hidden: false,
            toggled: None,
            numeric_value: None,
            min_numeric_value: None,
            max_numeric_value: None,
            step: None,
            actions: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn with_toggled(mut self, toggled: bool) -> Self {
        self.toggled = Some(toggled);
        self
    }

    pub fn with_numeric_range(mut self, value: f64, min: f64, max: f64, step: Option<f64>) -> Self {
        self.numeric_value = Some(value);
        self.min_numeric_value = Some(min);
        self.max_numeric_value = Some(max);
        self.step = step;
        self
    }

    pub fn with_action(mut self, action: Action) -> Self {
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
        self
    }
}
