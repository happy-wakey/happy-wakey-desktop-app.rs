import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

ApplicationWindow {
    id: window
    visible: true
    width: 1280
    height: 860
    title: "Happy Wakey"
    minimumWidth: 900
    minimumHeight: 600

    // The Rust state machine is the sole owner of onboarding completion.
    readonly property bool onboardingComplete: Backend.onboarding_complete

    // Pull onboarding state from Supabase once the UI is up (replaces the
    // startup hydrate that main() used to run before the engine existed).
    Component.onCompleted: Backend.startup()

    Theme { id: theme }

    // ---- Status Bar ----

    footer: ToolBar {
        height: 28
        background: Rectangle { color: theme.sidebarHeader }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: 8
            text: Backend.status_msg
            font.pixelSize: 11
            color: theme.muted
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            anchors.right: parent.right
            anchors.rightMargin: 8
            text: Backend.logged_in ? "✓ " + Backend.user_email : "Not logged in"
            font.pixelSize: 11
            color: Backend.logged_in ? theme.positive : theme.warning
        }
    }

    // ---- Main Layout ----

    RowLayout {
        id: mainLayout
        anchors.fill: parent
        spacing: 0
        visible: window.onboardingComplete

        // Sidebar
        Rectangle {
            id: sidebar
            Layout.preferredWidth: 200
            Layout.fillHeight: true
            color: theme.sidebar

            ColumnLayout {
                anchors.fill: parent
                spacing: 0

                // App title
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 56
                    color: theme.sidebarHeader

                    Text {
                        anchors.centerIn: parent
                        text: "☀ Happy Wakey"
                        font.pixelSize: 18
                        font.bold: true
                        color: theme.text
                    }
                }

                // Nav buttons
                Repeater {
                    model: [
                        { icon: "⌂", label: "Home",         panel: 0 },
                        { icon: "📅", label: "Calendar",     panel: 1 },
                        { icon: "🌤", label: "Weather",      panel: 2 },
                        { icon: "📈", label: "Stocks",       panel: 3 },
                        { icon: "📰", label: "News",         panel: 4 },
                        { icon: "🌐", label: "Browser",      panel: 5 },
                        { icon: "⚙",  label: "Settings",     panel: 6 },
                    ]

                    ItemDelegate {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 44
                        highlighted: stack.currentIndex === modelData.panel

                        contentItem: RowLayout {
                            spacing: 10
                            Text {
                                text: modelData.icon
                                font.pixelSize: 16
                            }
                            Text {
                                text: modelData.label
                                font.pixelSize: 14
                                color: highlighted ? theme.text : theme.muted
                            }
                        }

                        background: Rectangle {
                            color: highlighted ? theme.selected : "transparent"
                        }

                        onClicked: stack.currentIndex = modelData.panel
                    }
                }

                Item { Layout.fillHeight: true }
            }
        }

        // Content area
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: theme.page

            StackLayout {
                id: stack
                anchors.fill: parent
                anchors.margins: 16

                HomePanel {
                    theme: theme
                    onNavigate: function(panel) { stack.currentIndex = panel }
                }
                CalendarPanel { theme: theme }
                WeatherPanel { theme: theme }
                StocksPanel { theme: theme }
                NewsPanel { theme: theme }
                BrowserPanel { theme: theme }
                SettingsPanel { theme: theme }
            }
        }
    }

    OnboardingPanel {
        theme: theme
        anchors.fill: parent
        visible: !window.onboardingComplete
    }
}
