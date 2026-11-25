use crate::icon::IconSource;
use dbflux_core::{DatabaseCategory, Icon};

/// Declares every app icon once: the `AppIcon` variant, the asset path gpui
/// asks for, and the embedded bytes served for that path. One table keeps the
/// three in lockstep.
///
/// A file is relative to the workspace `resources/` directory and defaults to
/// the asset path; `from` names a different file.
macro_rules! app_icons {
    ($($icon:ident => $path:literal $(from $file:literal)?),* $(,)?) => {
        /// App-specific icons embedded from `resources/`, loaded through gpui's
        /// `AssetSource` by [`AppIcon::path`].
        ///
        /// ```rust,ignore
        /// Icon::new(AppIcon::Folder).size_4().color(theme.foreground)
        /// ```
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum AppIcon {
            $($icon,)*
        }

        impl AppIcon {
            pub const ALL: &[AppIcon] = &[$(AppIcon::$icon),*];

            /// Returns the asset path for this icon.
            pub const fn path(self) -> &'static str {
                match self {
                    $(Self::$icon => $path,)*
                }
            }

            /// Returns the embedded file served for [`AppIcon::path`].
            pub fn embedded_bytes(self) -> &'static [u8] {
                match self {
                    $(Self::$icon => include_bytes!(concat!(
                        "../../../../resources/",
                        icon_file!($path $(, $file)?)
                    )),)*
                }
            }
        }
    };
}

macro_rules! icon_file {
    ($path:literal) => {
        $path
    };
    ($path:literal, $file:literal) => {
        $file
    };
}

app_icons! {
    ChevronDown => "icons/ui/chevron-down.svg",
    ChevronLeft => "icons/ui/chevron-left.svg",
    ChevronRight => "icons/ui/chevron-right.svg",
    ChevronUp => "icons/ui/chevron-up.svg",
    Play => "icons/ui/play.svg",
    SquarePlay => "icons/ui/square-play.svg",
    Plus => "icons/ui/plus.svg",
    Power => "icons/ui/power.svg",
    Save => "icons/ui/save.svg",
    Delete => "icons/ui/delete.svg",
    Pencil => "icons/ui/pencil.svg",
    Copy => "icons/ui/copy.svg",
    RefreshCcw => "icons/ui/refresh-ccw.svg",
    RotateCcw => "icons/ui/rotate-ccw.svg",
    Download => "icons/ui/download.svg",
    Search => "icons/ui/search.svg",
    Settings => "icons/ui/settings.svg",
    History => "icons/ui/history.svg",
    Undo => "icons/ui/undo.svg",
    Redo => "icons/ui/redo.svg",
    X => "icons/ui/x.svg",
    Eye => "icons/ui/eye.svg",
    EyeOff => "icons/ui/eye-off.svg",
    Loader => "icons/ui/loader.svg",
    Info => "icons/ui/info.svg",
    CircleAlert => "icons/ui/circle-alert.svg",
    CircleCheck => "icons/ui/circle-check.svg",
    CircleX => "icons/ui/circle-x.svg",
    Check => "icons/ui/check.svg",
    ExternalLink => "icons/ui/external-link.svg",
    Globe => "icons/ui/globe.svg",
    TriangleAlert => "icons/ui/triangle-alert.svg",
    Code => "icons/ui/code.svg",
    Table => "icons/ui/table.svg",
    Columns => "icons/ui/columns.svg",
    Rows3 => "icons/ui/rows-3.svg",
    ArrowUp => "icons/ui/arrow-up.svg",
    ArrowDown => "icons/ui/arrow-down.svg",
    Star => "icons/ui/star.svg",
    Clock => "icons/ui/clock.svg",
    Zap => "icons/ui/zap.svg",
    Hash => "icons/ui/hash.svg",
    Lock => "icons/ui/lock.svg",
    Layers => "icons/ui/layers.svg",
    Keyboard => "icons/ui/keyboard.svg",
    FingerprintPattern => "icons/ui/fingerprint-pattern.svg",
    Maximize2 => "icons/ui/maximize-2.svg",
    Minimize2 => "icons/ui/minimize-2.svg",
    PanelBottomClose => "icons/ui/panel-bottom-close.svg",
    PanelBottomOpen => "icons/ui/panel-bottom-open.svg",
    PanelBottom => "icons/ui/panel-bottom.svg",
    PanelRight => "icons/ui/panel-right.svg",
    PanelRightClose => "icons/ui/panel-right-close.svg",
    PanelRightOpen => "icons/ui/panel-right-open.svg",
    FileSpreadsheet => "icons/ui/file-spreadsheet.svg",
    KeyRound => "icons/ui/key-round.svg",
    Cable => "icons/ui/cable.svg",
    Link2 => "icons/ui/link-2.svg",
    CaseSensitive => "icons/ui/case-sensitive.svg",
    ScrollText => "icons/ui/scroll-text.svg",
    ListFilter => "icons/ui/list-filter.svg",
    Tag => "icons/ui/tag.svg",
    Activity => "icons/ui/activity.svg",
    FileDown => "icons/ui/file-down.svg",
    FileScan => "icons/ui/file-scan.svg",
    SquareFunction => "icons/ui/square-function.svg",
    ArrowUpDown => "icons/ui/arrow-up-down.svg",
    Plug => "icons/ui/plug.svg",
    Unplug => "icons/ui/unplug.svg",
    Server => "icons/ui/server.svg",
    HardDrive => "icons/ui/hard-drive.svg",
    File => "icons/ui/file.svg",
    FileCode => "icons/ui/file-code-corner.svg",
    Image => "icons/ui/image.svg",
    Folder => "icons/ui/folder.svg",
    Box => "icons/ui/box.svg",
    Boxes => "icons/ui/boxes.svg",
    Braces => "icons/ui/braces.svg",
    SquareTerminal => "icons/ui/square-terminal.svg",
    Parentheses => "icons/ui/parentheses.svg",
    Sigma => "icons/ui/sigma.svg",
    Database => "icons/ui/database.svg",
    DatabaseZap => "icons/ui/database-zap.svg",
    ZoomIn => "icons/ui/zoom-in.svg",
    ZoomOut => "icons/ui/zoom-out.svg",
    Minus => "icons/ui/minus.svg",
    Grid3x3 => "icons/ui/grid-3x3.svg",
    Snowflake => "icons/ui/snowflake.svg",
    Scale => "icons/ui/scale.svg",
    ArrowLeftRight => "icons/ui/arrow-left-right.svg",
    Clipboard => "icons/ui/clipboard.svg",
    Pin => "icons/ui/pin.svg",
    Logs => "icons/ui/logs.svg",
    ChartSpline => "icons/ui/chart-spline.svg",
    ChartArea => "icons/ui/chart-area.svg",
    ChartColumnBig => "icons/ui/chart-column-big.svg",
    ChartNoAxesColumn => "icons/ui/chart-no-axes-column.svg",
    ChartBar => "icons/ui/chart-bar.svg",
    ChartPie => "icons/ui/chart-pie.svg",
    ChartScatter => "icons/ui/chart-scatter.svg",
    Crosshair => "icons/ui/crosshair.svg",
    ChartNetwork => "icons/ui/chart-network.svg",
    BrandPostgres => "icons/brand-color/postgresql.png",
    BrandMysql => "icons/brand-color/mysql.png",
    BrandMariadb => "icons/brand-color/mariadb.png",
    BrandSqlite => "icons/brand-color/sqlite.png",
    BrandMongodb => "icons/brand-color/mongodb.png",
    BrandRedis => "icons/brand-color/redis.png",
    BrandClickhouse => "icons/brand-color/clickhouse.png",
    BrandTurso => "icons/brand/turso.svg",
    BrandDuckdb => "icons/brand-color/duckdb.png",
    BrandDynamodb => "icons/brand-color/dynamodb.png",
    BrandLua => "icons/brand/lua.svg",
    BrandPython => "icons/brand/python.svg",
    BrandBash => "icons/brand/gnubash.svg",
    BrandJavaScript => "icons/brand/javascript.svg",
    BrandInfluxDb => "icons/brand-color/influxdb.png",
    BrandMssql => "icons/brand-color/mssql.png",
    DbFlux => "icons/dbflux.svg" from "branding/glyph.svg",
    BrainCircuit => "icons/ui/brain-circuit.svg",
    Bot => "icons/ui/bot.svg",
    Bell => "icons/ui/bell.svg",
}

impl AppIcon {
    /// Returns the best icon for a given query language.
    ///
    /// Languages with a dedicated brand SVG get their own icon. Languages that
    /// are DB-specific (SQL, MongoDB query, Redis, Cypher, CQL, InfluxQL) reuse
    /// the corresponding DB brand icon when one exists, or fall back to a
    /// generic file icon.
    pub fn for_language(lang: &dbflux_core::QueryLanguage) -> Self {
        use dbflux_core::QueryLanguage;
        match lang {
            QueryLanguage::Lua => Self::BrandLua,
            QueryLanguage::Python => Self::BrandPython,
            QueryLanguage::Bash => Self::BrandBash,
            QueryLanguage::MongoQuery => Self::BrandMongodb,
            QueryLanguage::RedisCommands => Self::BrandRedis,
            QueryLanguage::InfluxQuery | QueryLanguage::Flux => Self::BrandInfluxDb,
            QueryLanguage::Sql
            | QueryLanguage::CloudWatchLogsInsightsQl
            | QueryLanguage::OpenSearchPpl
            | QueryLanguage::OpenSearchSql
            | QueryLanguage::Cql => Self::Database,
            QueryLanguage::Cypher => Self::Database,
            QueryLanguage::Custom(_) => Self::FileCode,
        }
    }

    /// Returns the icon that best represents a given chart kind.
    ///
    /// Used for chart tabs and any chart-kind-specific affordance so the UI
    /// stays agnostic to the concrete `ChartKind` variants.
    pub const fn for_chart_kind(kind: crate::chart::ChartKind) -> Self {
        use crate::chart::ChartKind;
        match kind {
            ChartKind::Line => Self::ChartSpline,
            ChartKind::Bar => Self::ChartColumnBig,
            ChartKind::Scatter => Self::ChartNetwork,
            ChartKind::Area => Self::ChartArea,
            ChartKind::StackedBar => Self::ChartColumnBig,
            ChartKind::Pie => Self::ChartPie,
            ChartKind::Number => Self::Sigma,
        }
    }

    /// Resolves a driver icon, using its database category for generic fallbacks.
    pub const fn for_driver(icon: Icon, category: DatabaseCategory) -> Self {
        match icon {
            Icon::Postgres => Self::BrandPostgres,
            Icon::Mysql => Self::BrandMysql,
            Icon::Mariadb => Self::BrandMariadb,
            Icon::Sqlite => Self::BrandSqlite,
            Icon::Mongodb => Self::BrandMongodb,
            Icon::Redis => Self::BrandRedis,
            Icon::Dynamodb => Self::BrandDynamodb,
            Icon::Redshift => Self::ChartNoAxesColumn,
            Icon::S3 => Self::Boxes,
            Icon::Clickhouse => Self::BrandClickhouse,
            Icon::Turso => Self::BrandTurso,
            Icon::Duckdb => Self::BrandDuckdb,
            Icon::Mssql => Self::BrandMssql,
            Icon::Influxdb => Self::BrandInfluxDb,
            Icon::Logs => Self::Logs,
            Icon::Database => crate::category::category_presentation(category).fallback_icon,
        }
    }
}

/// Palette role of a driver logo, from the driver-declared [`Icon`] and its
/// database category, never from a driver id (P1Sidebar, P1DriverPicker):
/// relational engines read in the info blue, document stores in green,
/// key-value stores in the danger red, time series in the NULL violet, and
/// object storage and log streams in amber.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverIconTone {
    Info,
    Success,
    Danger,
    Violet,
    Warning,
}

impl DriverIconTone {
    pub const fn for_driver(icon: Icon, category: DatabaseCategory) -> Self {
        match icon {
            Icon::Postgres
            | Icon::Mysql
            | Icon::Mariadb
            | Icon::Sqlite
            | Icon::Redshift
            | Icon::Clickhouse
            | Icon::Turso
            | Icon::Duckdb
            | Icon::Mssql => Self::Info,
            Icon::Mongodb | Icon::Dynamodb => Self::Success,
            Icon::Redis => Self::Danger,
            Icon::Influxdb => Self::Violet,
            Icon::S3 | Icon::Logs => Self::Warning,
            Icon::Database => crate::category::category_presentation(category).fallback_tone,
        }
    }

    /// The same tone as a row or badge tone, which resolves its own color
    /// against the theme when it draws. Use it where a color is stored and
    /// drawn later, so a light and dark switch does not leave a stale hue.
    pub const fn badge_tone(self) -> crate::primitives::BadgeTone {
        use crate::primitives::BadgeTone;

        match self {
            Self::Info => BadgeTone::Info,
            Self::Success => BadgeTone::Success,
            Self::Danger => BadgeTone::Danger,
            Self::Violet => BadgeTone::Violet,
            Self::Warning => BadgeTone::Warning,
        }
    }

    /// The color of this tone in the active theme.
    pub fn resolve(self, cx: &gpui::App) -> gpui::Hsla {
        use gpui_component::ActiveTheme;

        let theme = cx.theme();

        match self {
            Self::Info => theme.info,
            Self::Success => theme.success,
            Self::Danger => theme.danger,
            Self::Violet => crate::tokens::SyntaxColors::for_current(cx).number,
            Self::Warning => theme.warning,
        }
    }
}

impl From<AppIcon> for IconSource {
    fn from(icon: AppIcon) -> Self {
        IconSource::Svg(icon.path().into())
    }
}

impl From<AppIcon> for gpui_component::Icon {
    fn from(icon: AppIcon) -> Self {
        gpui_component::Icon::default().path(icon.path())
    }
}

#[cfg(test)]
mod tests {
    use super::{AppIcon, DriverIconTone};

    #[test]
    fn driver_tones_follow_the_declared_icon_then_the_category() {
        assert_eq!(
            DriverIconTone::for_driver(Icon::Postgres, DatabaseCategory::Relational),
            DriverIconTone::Info
        );
        assert_eq!(
            DriverIconTone::for_driver(Icon::Redis, DatabaseCategory::KeyValue),
            DriverIconTone::Danger
        );
        assert_eq!(
            DriverIconTone::for_driver(Icon::S3, DatabaseCategory::ObjectStorage),
            DriverIconTone::Warning
        );
        assert_eq!(
            DriverIconTone::for_driver(Icon::Database, DatabaseCategory::Document),
            DriverIconTone::Success
        );
        assert_eq!(
            DriverIconTone::for_driver(Icon::Database, DatabaseCategory::TimeSeries),
            DriverIconTone::Violet
        );
    }
    use dbflux_core::{DatabaseCategory, Icon};

    #[test]
    fn explicit_driver_icons_preserve_brands_and_semantic_fallbacks() {
        assert_eq!(
            AppIcon::for_driver(Icon::Influxdb, DatabaseCategory::TimeSeries).path(),
            "icons/brand-color/influxdb.png"
        );
        assert_eq!(
            AppIcon::for_driver(Icon::Redshift, DatabaseCategory::Relational),
            AppIcon::ChartNoAxesColumn
        );
        assert_eq!(
            AppIcon::for_driver(Icon::Dynamodb, DatabaseCategory::Document),
            AppIcon::BrandDynamodb
        );
        assert_eq!(
            AppIcon::for_driver(Icon::S3, DatabaseCategory::ObjectStorage),
            AppIcon::Boxes
        );
        assert_eq!(
            AppIcon::for_driver(Icon::Clickhouse, DatabaseCategory::Relational),
            AppIcon::BrandClickhouse
        );
        assert_eq!(
            AppIcon::for_driver(Icon::Turso, DatabaseCategory::Relational),
            AppIcon::BrandTurso
        );
    }

    #[test]
    fn generic_database_icon_uses_category_fallbacks() {
        let cases = [
            (DatabaseCategory::Relational, AppIcon::Database),
            (DatabaseCategory::Document, AppIcon::Braces),
            (DatabaseCategory::KeyValue, AppIcon::KeyRound),
            (DatabaseCategory::ObjectStorage, AppIcon::Boxes),
            (DatabaseCategory::TimeSeries, AppIcon::Database),
            (DatabaseCategory::LogStream, AppIcon::Database),
            (DatabaseCategory::SecretStore, AppIcon::Lock),
        ];

        for (category, expected) in cases {
            assert_eq!(AppIcon::for_driver(Icon::Database, category), expected);
        }
    }

    #[test]
    fn semantic_driver_icons_use_expected_asset_paths() {
        assert_eq!(
            AppIcon::ChartNoAxesColumn.path(),
            "icons/ui/chart-no-axes-column.svg"
        );
        assert_eq!(AppIcon::Boxes.path(), "icons/ui/boxes.svg");
    }
}
