//! Shared data types for the resource table, ported from
//! `../halreslib-iced/src/model.rs` (minus serde, which elmore's
//! `include!`-ed dataset doesn't need).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Health,
    Host,
    Title,
    Url,
    Description,
    ManualDescription,
    Tags,
    Scheme,
    Path,
    Uuid,
    CreatedBy,
    CreatedAt,
    ModifiedBy,
    ModifiedAt,
}

impl Column {
    pub const ALL: [Self; 14] = [
        Self::Health,
        Self::Host,
        Self::Title,
        Self::Url,
        Self::Description,
        Self::ManualDescription,
        Self::Tags,
        Self::Scheme,
        Self::Path,
        Self::Uuid,
        Self::CreatedBy,
        Self::CreatedAt,
        Self::ModifiedBy,
        Self::ModifiedAt,
    ];

    pub const DEFAULT_VISIBLE: [Self; 4] = [Self::Host, Self::Title, Self::Url, Self::Description];

    pub fn count() -> usize {
        Self::ALL.len()
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Health => "Health",
            Self::Host => "Domain",
            Self::Title => "Title",
            Self::Url => "URL",
            Self::Description => "Description",
            Self::ManualDescription => "Manual description",
            Self::Tags => "Tags",
            Self::Scheme => "Scheme",
            Self::Path => "Path",
            Self::Uuid => "Identifier",
            Self::CreatedBy => "Created by",
            Self::CreatedAt => "Created at",
            Self::ModifiedBy => "Modified by",
            Self::ModifiedAt => "Modified at",
        }
    }

    /// Columns whose content is short and rendered centered
    pub fn is_compact(self) -> bool {
        matches!(self, Self::Health | Self::Tags | Self::Scheme)
    }

    /// Get the index of this column in the enum definition
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|item| *item == self).unwrap()
    }

    pub fn value(self, uri: &Uri) -> String {
        match self {
            Self::Health => uri.live_status.label().to_string(),
            Self::Host => uri.host.clone().unwrap_or_default(),
            Self::Title => uri.title.clone().unwrap_or_default(),
            Self::Url => uri.url.clone(),
            Self::Description => uri.auto_descr.clone().unwrap_or_default(),
            Self::ManualDescription => uri.man_descr.clone().unwrap_or_default(),
            Self::Tags => uri.tags.join(", "),
            Self::Scheme => uri.scheme.clone(),
            Self::Path => uri.path.clone().unwrap_or_default(),
            Self::Uuid => uri.uri_uuid.clone(),
            Self::CreatedBy => uri.crea_user.clone().unwrap_or_default(),
            Self::CreatedAt => uri.crea_time.clone().unwrap_or_default(),
            Self::ModifiedBy => uri.modi_user.clone().unwrap_or_default(),
            Self::ModifiedAt => uri.modi_time.clone().unwrap_or_default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortRule {
    pub column: Column,
    pub direction: SortDirection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TablePreferences {
    pub sort_rules: Vec<SortRule>,
    pub column_order: Vec<Column>,
    pub visible_columns: Vec<Column>,
}

impl Default for TablePreferences {
    fn default() -> Self {
        Self {
            sort_rules: Vec::new(),
            column_order: Column::ALL.to_vec(),
            visible_columns: Column::DEFAULT_VISIBLE.to_vec(),
        }
    }
}

impl TablePreferences {
    pub fn visible_in_order(&self) -> Vec<Column> {
        self.column_order
            .iter()
            .copied()
            .filter(|column| self.visible_columns.contains(column))
            .collect()
    }

    pub fn toggle_column(&mut self, column: Column) {
        if let Some(index) = self.visible_columns.iter().position(|c| *c == column) {
            self.visible_columns.remove(index);
        } else {
            self.visible_columns.push(column);
        }
    }

    pub fn is_column_visible(&self, column: Column) -> bool {
        self.visible_columns.contains(&column)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Available,
    Unavailable,
    Unknown,
}

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => "Available",
            Self::Unavailable => "Unavailable",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Uri {
    pub uri_uuid: String,
    pub url: String,
    pub scheme: String,
    pub host: Option<String>,
    pub path: Option<String>,
    pub live_status: Health,
    pub title: Option<String>,
    pub auto_descr: Option<String>,
    pub man_descr: Option<String>,
    pub crea_user: Option<String>,
    pub crea_time: Option<String>,
    pub modi_user: Option<String>,
    pub modi_time: Option<String>,
    pub tags: Vec<String>,
}
