use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new()
        .qt_module("Quick")
        .qml_module(QmlModule {
            uri: "com.happywakey",
            version_major: 1,
            version_minor: 0,
            // The bridge (Backend QObject) lives in main.rs.
            rust_files: &["src/main.rs"],
            qml_files: &[
                "qml/MainWindow.qml",
                "qml/Theme.qml",
                "qml/HomePanel.qml",
                "qml/CalendarPanel.qml",
                "qml/WeatherPanel.qml",
                "qml/StocksPanel.qml",
                "qml/NewsPanel.qml",
                "qml/PlannerPanel.qml",
                "qml/FocusPanel.qml",
                "qml/DevicesPanel.qml",
                "qml/BrowserPanel.qml",
                "qml/SettingsPanel.qml",
                "qml/OnboardingPanel.qml",
            ],
            ..Default::default()
        })
        .build();
}
