use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThemeMode {
    Cyberpunk,
    Dracula,
    Nord,
    Monokai,
    Gruvbox,
}

impl ThemeMode {
    pub fn next(&self) -> Self {
        match self {
            Self::Cyberpunk => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::Monokai,
            Self::Monokai => Self::Gruvbox,
            Self::Gruvbox => Self::Cyberpunk,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Cyberpunk => "Cyberpunk",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::Monokai => "Monokai",
            Self::Gruvbox => "Gruvbox",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub border: Color,
    #[allow(dead_code)]
    pub border_active: Color,
    pub card_bg: Color,
    pub selected_bg: Color,
    pub text_dim: Color,
}

impl Theme {
    pub fn new(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Cyberpunk => Self {
                mode,
                bg: Color::Rgb(15, 15, 26),
                fg: Color::Rgb(240, 240, 255),
                accent: Color::Rgb(0, 240, 255),    // Neon Cyan
                secondary: Color::Rgb(255, 0, 127), // Neon Pink
                success: Color::Rgb(0, 255, 136),   // Neon Green
                warning: Color::Rgb(255, 215, 0),   // Neon Gold
                danger: Color::Rgb(255, 50, 75),    // Hot Red
                border: Color::Rgb(50, 50, 80),
                border_active: Color::Rgb(0, 240, 255),
                card_bg: Color::Rgb(24, 24, 40),
                selected_bg: Color::Rgb(40, 35, 75),
                text_dim: Color::Rgb(130, 130, 170),
            },
            ThemeMode::Dracula => Self {
                mode,
                bg: Color::Rgb(40, 42, 54),
                fg: Color::Rgb(248, 248, 242),
                accent: Color::Rgb(189, 147, 249),    // Purple
                secondary: Color::Rgb(255, 121, 198), // Pink
                success: Color::Rgb(80, 250, 123),    // Green
                warning: Color::Rgb(241, 250, 140),   // Yellow
                danger: Color::Rgb(255, 85, 85),      // Red
                border: Color::Rgb(68, 71, 90),
                border_active: Color::Rgb(189, 147, 249),
                card_bg: Color::Rgb(50, 52, 68),
                selected_bg: Color::Rgb(75, 78, 102),
                text_dim: Color::Rgb(140, 145, 170),
            },
            ThemeMode::Nord => Self {
                mode,
                bg: Color::Rgb(46, 52, 64),
                fg: Color::Rgb(236, 239, 244),
                accent: Color::Rgb(136, 192, 208),    // Frost Blue
                secondary: Color::Rgb(129, 161, 193), // Deep Frost
                success: Color::Rgb(163, 190, 140),   // Aurora Green
                warning: Color::Rgb(235, 203, 139),   // Aurora Yellow
                danger: Color::Rgb(191, 97, 106),     // Aurora Red
                border: Color::Rgb(76, 86, 106),
                border_active: Color::Rgb(136, 192, 208),
                card_bg: Color::Rgb(59, 66, 82),
                selected_bg: Color::Rgb(76, 86, 106),
                text_dim: Color::Rgb(145, 155, 175),
            },
            ThemeMode::Monokai => Self {
                mode,
                bg: Color::Rgb(39, 40, 34),
                fg: Color::Rgb(248, 248, 242),
                accent: Color::Rgb(102, 217, 239),   // Cyan
                secondary: Color::Rgb(249, 38, 114), // Magenta
                success: Color::Rgb(166, 226, 46),   // Green
                warning: Color::Rgb(253, 151, 31),   // Orange
                danger: Color::Rgb(249, 38, 114),    // Red
                border: Color::Rgb(73, 72, 62),
                border_active: Color::Rgb(166, 226, 46),
                card_bg: Color::Rgb(50, 51, 44),
                selected_bg: Color::Rgb(75, 75, 65),
                text_dim: Color::Rgb(140, 140, 130),
            },
            ThemeMode::Gruvbox => Self {
                mode,
                bg: Color::Rgb(40, 40, 40),
                fg: Color::Rgb(235, 219, 178),
                accent: Color::Rgb(254, 128, 25), // Bright Orange
                secondary: Color::Rgb(211, 134, 155), // Bright Purple
                success: Color::Rgb(184, 187, 38), // Bright Green
                warning: Color::Rgb(250, 189, 47), // Bright Yellow
                danger: Color::Rgb(251, 73, 52),  // Bright Red
                border: Color::Rgb(80, 73, 69),
                border_active: Color::Rgb(254, 128, 25),
                card_bg: Color::Rgb(50, 48, 47),
                selected_bg: Color::Rgb(80, 73, 69),
                text_dim: Color::Rgb(168, 153, 132),
            },
        }
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.card_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.text_dim)
    }
}
