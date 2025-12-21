/// Configuration for debug visualization
#[derive(Clone, Copy, Debug, Default)]
pub struct DebugOptions {
    /// Show margin areas (red overlay)
    pub show_margins: bool,
    /// Show padding areas (blue overlay)
    pub show_padding: bool,
    /// Show node borders (green outline)
    pub show_borders: bool,
    /// Show content areas (yellow outline)
    pub show_content_area: bool,
}

impl DebugOptions {
    /// Create debug options with nothing enabled
    pub const fn none() -> Self {
        Self {
            show_margins: false,
            show_padding: false,
            show_borders: false,
            show_content_area: false,
        }
    }

    /// Create debug options with all visualizations enabled
    pub const fn all() -> Self {
        Self {
            show_margins: true,
            show_padding: true,
            show_borders: true,
            show_content_area: true,
        }
    }

    /// Enable margin visualization
    pub const fn with_margins(mut self, enabled: bool) -> Self {
        self.show_margins = enabled;
        self
    }

    /// Enable padding visualization
    pub const fn with_padding(mut self, enabled: bool) -> Self {
        self.show_padding = enabled;
        self
    }

    /// Enable border visualization
    pub const fn with_borders(mut self, enabled: bool) -> Self {
        self.show_borders = enabled;
        self
    }

    /// Enable content area visualization
    pub const fn with_content_area(mut self, enabled: bool) -> Self {
        self.show_content_area = enabled;
        self
    }

    /// Check if any debug visualization is enabled
    pub const fn is_enabled(&self) -> bool {
        self.show_margins || self.show_padding || self.show_borders || self.show_content_area
    }
}
