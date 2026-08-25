import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    id: root
    color: theme.page

    property var theme
    property var steps: [
        { id: "welcome", title: "Start the day in one place", kicker: "Happy Wakey keeps the daily essentials close without turning your desktop into tab soup." },
        { id: "account", title: "Connect calendar sync", kicker: "Sign in once, then calendar reminders and cloud onboarding progress can follow you." },
        { id: "backup", title: "Choose your backup home", kicker: "Use a private git repo path for portable JSON config, with Supabase holding onboarding progress." },
        { id: "essentials", title: "Pick your first essentials", kicker: "Seed weather, markets, news, and browser shortcuts. You can refine these later." },
        { id: "ready", title: "You are ready", kicker: "Your workspace is set up. The app will open straight into the dashboard next time." }
    ]

    readonly property int stepIndex: Backend.onboarding_step_index
    property var cfg: parseConfig()
    property string actionStatus: ""

    function parseConfig() {
        try {
            return JSON.parse(Backend.app_config_json)
        } catch(e) {
            return {}
        }
    }

    function nextStep() {
        if (stepIndex < steps.length - 1) {
            actionStatus = "Saving next setup step..."
            Backend.onboarding_next()
        }
    }

    function previousStep() {
        if (stepIndex > 0) {
            actionStatus = "Saving previous setup step..."
            Backend.onboarding_previous()
        }
    }

    function skipToReady() {
        actionStatus = "Saving ready state..."
        Backend.onboarding_skip_to_ready()
    }

    function finishOnboarding() {
        actionStatus = "Opening dashboard..."
        applyStarterConfig()
        Backend.onboarding_finish()
    }

    function applyStarterConfig() {
        cfg = parseConfig()
        cfg.git_repo_path = gitRepoPath.text.trim()

        var locationName = locationNameInput.text.trim()
        var lat = parseFloat(latInput.text)
        var lon = parseFloat(lonInput.text)
        if (locationName.length > 0 && !isNaN(lat) && !isNaN(lon)) {
            cfg.weather_locations = [{ name: locationName, lat: lat, lon: lon }]
        }

        var symbols = stockInput.text.split(",").map(function(s) { return s.trim().toUpperCase() }).filter(function(s) { return s.length > 0 })
        if (symbols.length > 0) cfg.stock_symbols = symbols.slice(0, 20)

        var keywords = keywordInput.text.split(",").map(function(s) { return s.trim() }).filter(function(s) { return s.length > 0 })
        if (keywords.length > 0) cfg.news_keywords = keywords

        var shortcut = shortcutUrl.text.trim()
        if (shortcut.length > 0) {
            cfg.browser_bookmarks = [{
                id: "primary",
                title: shortcutTitle.text.trim() || shortcut,
                url: shortcut
            }]
        }

        Backend.save_config(JSON.stringify(cfg))
    }

    Rectangle {
        anchors.fill: parent
        color: theme.page
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 44
        spacing: 36

        Rectangle {
            Layout.preferredWidth: 280
            Layout.fillHeight: true
            color: theme.sidebar
            radius: 8
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 24
                spacing: 18

                Text {
                    text: "Happy Wakey"
                    color: theme.text
                    font.pixelSize: 24
                    font.bold: true
                }

                Text {
                    text: "Setup progress"
                    color: theme.muted
                    font.pixelSize: 12
                }

                Repeater {
                    model: steps
                    delegate: RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        Rectangle {
                            Layout.preferredWidth: 28
                            Layout.preferredHeight: 28
                            radius: 14
                            color: index <= stepIndex ? theme.accentSoft : theme.surface
                            border.color: index === stepIndex ? theme.accent : theme.border
                            Text {
                                anchors.centerIn: parent
                                text: String(index + 1)
                                color: index <= stepIndex ? theme.accent : theme.muted
                                font.pixelSize: 12
                                font.bold: true
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: modelData.title
                            color: index === stepIndex ? theme.text : theme.muted
                            font.pixelSize: 13
                            wrapMode: Text.WordWrap
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                Text {
                    Layout.fillWidth: true
                    text: Backend.logged_in ? "Signed in as " + Backend.user_email : "Local setup is available before sign-in."
                    color: Backend.logged_in ? theme.positive : theme.muted
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: theme.content
            radius: 8
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 34
                spacing: 20

                Text {
                    Layout.fillWidth: true
                    text: steps[stepIndex].title
                    color: theme.text
                    font.pixelSize: 34
                    font.bold: true
                    wrapMode: Text.WordWrap
                }

                Text {
                    Layout.fillWidth: true
                    text: steps[stepIndex].kicker
                    color: theme.muted
                    font.pixelSize: 15
                    lineHeight: 1.25
                    wrapMode: Text.WordWrap
                }

                StackLayout {
                    id: pages
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    currentIndex: stepIndex

                    ColumnLayout {
                        spacing: 14
                        Text { text: "Daily command center"; color: theme.accent; font.pixelSize: 14; font.bold: true }
                        Text { Layout.fillWidth: true; text: "Calendar, weather, markets, headlines, and your most-used pages are arranged as a quiet desktop cockpit. The setup state is saved as you move."; color: theme.text; font.pixelSize: 16; wrapMode: Text.WordWrap }
                    }

                    ColumnLayout {
                        spacing: 12
                        Text { Layout.fillWidth: true; text: "Use Google, Apple, or Microsoft. Microsoft is routed through Supabase's Azure provider."; color: theme.text; font.pixelSize: 15; wrapMode: Text.WordWrap }
                        RowLayout {
                            spacing: 10
                            Button { text: "Google"; enabled: !Backend.auth_busy; onClicked: Backend.login("google") }
                            Button { text: "Apple"; enabled: !Backend.auth_busy; onClicked: Backend.login("apple") }
                            Button { text: "Microsoft"; enabled: !Backend.auth_busy; onClicked: Backend.login("microsoft") }
                        }
                    }

                    ColumnLayout {
                        spacing: 12
                        Text { Layout.fillWidth: true; text: "Private git repo or local path"; color: theme.text; font.pixelSize: 15 }
                        TextField {
                            id: gitRepoPath
                            Layout.fillWidth: true
                            placeholderText: "git@github.com:user/happy-wakey-config.git"
                            text: (cfg.git_repo_path || "")
                            selectByMouse: true
                        }
                    }

                    ColumnLayout {
                        spacing: 12
                        GridLayout {
                            Layout.fillWidth: true
                            columns: 2
                            rowSpacing: 10
                            columnSpacing: 10

                            TextField { id: locationNameInput; Layout.fillWidth: true; placeholderText: "Weather location"; text: "Chicago" }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                TextField { id: latInput; Layout.fillWidth: true; placeholderText: "Lat"; text: "41.8781"; validator: DoubleValidator {} }
                                TextField { id: lonInput; Layout.fillWidth: true; placeholderText: "Lon"; text: "-87.6298"; validator: DoubleValidator {} }
                            }
                            TextField { id: stockInput; Layout.fillWidth: true; placeholderText: "Stocks"; text: "AAPL, MSFT, NVDA, SPY, QQQ" }
                            TextField { id: keywordInput; Layout.fillWidth: true; placeholderText: "News keywords"; text: "technology, AI, markets" }
                            TextField { id: shortcutTitle; Layout.fillWidth: true; placeholderText: "Shortcut title"; text: "Inbox" }
                            TextField { id: shortcutUrl; Layout.fillWidth: true; placeholderText: "https://mail.google.com" }
                        }
                    }

                    ColumnLayout {
                        spacing: 14
                        Text { Layout.fillWidth: true; text: "Setup is saved locally and, after sign-in, mirrored to Supabase onboarding state."; color: theme.text; font.pixelSize: 16; wrapMode: Text.WordWrap }
                        Text { Layout.fillWidth: true; text: "The dashboard will open next. You can revisit every choice in Settings."; color: theme.muted; font.pixelSize: 14; wrapMode: Text.WordWrap }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 48
                    spacing: 10
                    z: 10

                    NavButton {
                        text: "Back"
                        enabled: stepIndex > 0
                        theme: root.theme
                        primary: false
                        onClicked: previousStep()
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: actionStatus
                        color: theme.muted
                        font.pixelSize: 12
                        visible: actionStatus.length > 0
                    }

                    NavButton {
                        text: "Skip setup"
                        visible: stepIndex < steps.length - 1
                        theme: root.theme
                        primary: false
                        onClicked: skipToReady()
                    }

                    NavButton {
                        text: stepIndex === steps.length - 1 ? "Open dashboard" : "Continue"
                        theme: root.theme
                        primary: true
                        onClicked: {
                            if (stepIndex === 2 || stepIndex === 3) applyStarterConfig()
                            if (stepIndex === steps.length - 1) {
                                finishOnboarding()
                            } else {
                                nextStep()
                            }
                        }
                    }
                }
            }
        }
    }

    component NavButton: Rectangle {
        id: button

        property string text: ""
        property bool primary: false
        property var theme
        signal clicked()

        Layout.preferredWidth: Math.max(132, label.implicitWidth + 34)
        Layout.preferredHeight: 44
        radius: 6
        opacity: enabled ? 1 : 0.45
        color: primary
            ? (mouseArea.pressed ? Qt.darker(theme.accent, 1.15) : theme.accent)
            : (mouseArea.pressed ? theme.selected : theme.surfaceAlt)
        border.color: primary ? theme.accent : theme.border
        border.width: 1

        Text {
            id: label
            anchors.centerIn: parent
            text: button.text
            color: button.primary ? theme.accentText : theme.text
            font.pixelSize: 13
            font.bold: button.primary
        }

        MouseArea {
            id: mouseArea
            anchors.fill: parent
            enabled: button.enabled
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: button.clicked()
        }
    }
}
