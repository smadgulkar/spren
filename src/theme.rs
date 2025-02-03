pub use colored::{Color, Colorize};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColorScheme {
    pub foreground: Option<ColorWrapper>,
    pub background: Option<ColorWrapper>,
    pub bold: bool,
    pub underline: bool,
}

// Wrapper type for Color that implements Serialize/Deserialize
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
pub struct ColorWrapper(Color);

impl From<ColorWrapper> for String {
    fn from(wrapper: ColorWrapper) -> Self {
        format!("{:?}", wrapper.0)
    }
}

impl From<String> for ColorWrapper {
    fn from(s: String) -> Self {
        ColorWrapper(Color::from_str(&s).unwrap_or(Color::White))
    }
}

impl From<Color> for ColorWrapper {
    fn from(color: Color) -> Self {
        ColorWrapper(color)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub header: ColorScheme,
    pub explanation: ColorScheme,
    pub command: ColorScheme,
    pub warning: ColorScheme,
    pub rollback: ColorScheme,
    pub impact: ColorScheme,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            header: ColorScheme {
                foreground: Some(ColorWrapper(Color::Blue)),
                background: None,
                bold: true,
                underline: false,
            },
            explanation: ColorScheme {
                foreground: Some(ColorWrapper(Color::Yellow)),
                background: None,
                bold: false,
                underline: false,
            },
            command: ColorScheme {
                foreground: Some(ColorWrapper(Color::Green)),
                background: None,
                bold: false,
                underline: false,
            },
            warning: ColorScheme {
                foreground: Some(ColorWrapper(Color::Red)),
                background: None,
                bold: true,
                underline: false,
            },
            rollback: ColorScheme {
                foreground: Some(ColorWrapper(Color::Red)),
                background: None,
                bold: false,
                underline: false,
            },
            impact: ColorScheme {
                foreground: Some(ColorWrapper(Color::Cyan)),
                background: None,
                bold: false,
                underline: false,
            },
        }
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self::default()
    }

    pub fn light() -> Self {
        Self {
            header: ColorScheme {
                foreground: Some(ColorWrapper(Color::BrightBlue)),
                ..Default::default()
            },
            explanation: ColorScheme {
                foreground: Some(ColorWrapper(Color::BrightYellow)),
                ..Default::default()
            },
            command: ColorScheme {
                foreground: Some(ColorWrapper(Color::BrightGreen)),
                ..Default::default()
            },
            warning: ColorScheme {
                foreground: Some(ColorWrapper(Color::BrightRed)),
                ..Default::default()
            },
            rollback: ColorScheme {
                foreground: Some(ColorWrapper(Color::BrightRed)),
                ..Default::default()
            },
            impact: ColorScheme {
                foreground: Some(ColorWrapper(Color::BrightCyan)),
                ..Default::default()
            },
        }
    }

    pub fn monochrome() -> Self {
        Self {
            header: ColorScheme {
                foreground: None,
                background: None,
                bold: true,
                underline: true,
            },
            explanation: ColorScheme {
                foreground: None,
                background: None,
                bold: true,
                underline: false,
            },
            command: ColorScheme {
                foreground: None,
                background: None,
                bold: false,
                underline: false,
            },
            warning: ColorScheme {
                foreground: None,
                background: None,
                bold: true,
                underline: true,
            },
            rollback: ColorScheme {
                foreground: None,
                background: None,
                bold: true,
                underline: false,
            },
            impact: ColorScheme {
                foreground: None,
                background: None,
                bold: false,
                underline: false,
            },
        }
    }
}

impl ColorScheme {
    pub fn apply(&self, text: &str) -> colored::ColoredString {
        let mut colored_text: colored::ColoredString = text.into();

        if let Some(fg) = &self.foreground {
            colored_text = colored_text.color(fg.0);
        }
        if let Some(bg) = &self.background {
            colored_text = colored_text.on_color(bg.0);
        }
        if self.bold {
            colored_text = colored_text.bold();
        }
        if self.underline {
            colored_text = colored_text.underline();
        }

        colored_text
    }
} 