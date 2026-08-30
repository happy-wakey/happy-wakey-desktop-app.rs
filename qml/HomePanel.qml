import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    id: root
    color: "transparent"

    property var theme
    signal navigate(int panel)

    property var cfg: ({})
    property int weatherLocationCount: 0
    property int stockSymbolCount: 0
    property int newsKeywordCount: 0
    property int bookmarkCount: 0
    property var calendarAgenda: ({ total_events: 0, headline: "Weekly events and reminders" })
    property bool anyLoading: Backend.calendar_loading || Backend.weather_loading
        || Backend.stocks_loading || Backend.news_loading

    function parseJson(json, fallback) {
        try {
            if (!json || json.length === 0) return fallback
            return JSON.parse(json)
        } catch(e) {
            return fallback
        }
    }

    function refreshConfig() {
        cfg = parseJson(Backend.app_config_json, {})
        weatherLocationCount = cfg.weather_locations ? cfg.weather_locations.length : 0
        stockSymbolCount = cfg.stock_symbols ? cfg.stock_symbols.length : 0
        newsKeywordCount = cfg.news_keywords ? cfg.news_keywords.length : 0
        bookmarkCount = cfg.browser_bookmarks ? cfg.browser_bookmarks.length : 0
        rebuildBookmarks()
    }

    function rebuildCalendar() {
        var arr = parseJson(Backend.calendar_json, [])
        calendarAgenda = parseJson(Backend.calendar_agenda_json, calendarAgenda)
        calendarModel.clear()
        for (var i = 0; i < Math.min(arr.length, 3); i++) {
            var ev = arr[i]
            calendarModel.append({
                title: ev.title || "Untitled",
                meta: (ev.day_label || "Upcoming") + " · " + (ev.time_label || "Anytime")
            })
        }
    }

    function rebuildWeather() {
        var arr = parseJson(Backend.weather_json, [])
        weatherModel.clear()
        for (var i = 0; i < Math.min(arr.length, 3); i++) {
            var w = arr[i]
            weatherModel.append({
                title: w.location_name || "Unknown",
                meta: Math.round(w.temperature) + "° · " + (w.condition || "")
            })
        }
    }

    function rebuildStocks() {
        var arr = parseJson(Backend.stocks_json, [])
        stocksModel.clear()
        for (var i = 0; i < Math.min(arr.length, 4); i++) {
            var s = arr[i]
            stocksModel.append({
                title: s.symbol || "",
                meta: "$" + Number(s.price || 0).toFixed(2) + " · " + Number(s.change_percent || 0).toFixed(2) + "%"
            })
        }
    }

    function rebuildNews() {
        var arr = parseJson(Backend.news_json, [])
        newsModel.clear()
        for (var i = 0; i < Math.min(arr.length, 3); i++) {
            var n = arr[i]
            newsModel.append({
                title: n.title || "",
                meta: n.source || "News"
            })
        }
    }

    function rebuildBookmarks() {
        bookmarkModel.clear()
        var arr = cfg.browser_bookmarks || []
        for (var i = 0; i < Math.min(arr.length, 3); i++) {
            var b = arr[i]
            bookmarkModel.append({
                title: b.title || b.url || "Bookmark",
                meta: b.url || ""
            })
        }
    }

    function refreshAll() {
        Backend.refresh_calendar()
        Backend.refresh_weather()
        Backend.refresh_stocks()
        Backend.refresh_news()
    }

    Component.onCompleted: {
        refreshConfig()
        rebuildCalendar()
        rebuildWeather()
        rebuildStocks()
        rebuildNews()
    }

    Connections {
        target: Backend
        function onApp_config_jsonChanged() { refreshConfig() }
        function onCalendar_jsonChanged() { rebuildCalendar() }
        function onCalendar_agenda_jsonChanged() { rebuildCalendar() }
        function onWeather_jsonChanged() { rebuildWeather() }
        function onStocks_jsonChanged() { rebuildStocks() }
        function onNews_jsonChanged() { rebuildNews() }
    }

    ListModel { id: calendarModel }
    ListModel { id: weatherModel }
    ListModel { id: stocksModel }
    ListModel { id: newsModel }
    ListModel { id: bookmarkModel }
    ListModel { id: bluetoothPreviewModel }

    ColumnLayout {
        anchors.fill: parent
        spacing: 14

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Home"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: Backend.logged_in ? "Signed in as " + Backend.user_email : "Local dashboard preview"
                    font.pixelSize: 12
                    color: theme.muted
                }
            }

            Button {
                text: root.anyLoading ? "Refreshing..." : "Refresh all"
                enabled: !root.anyLoading
                onClicked: refreshAll()
            }
        }

        ScrollView {
            id: homeScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            GridLayout {
                id: homeGrid
                width: homeScroll.availableWidth
                columns: width > 980 ? 3 : 2
                property real cardWidth: Math.max(300,
                    (width - columnSpacing * (columns - 1)) / columns)
                columnSpacing: 12
                rowSpacing: 12

                PreviewCard {
                    theme: root.theme
                    Layout.preferredWidth: homeGrid.cardWidth
                    Layout.preferredHeight: 248
                    title: "Calendar"
                    metric: calendarAgenda.total_events > 0 ? calendarAgenda.total_events + " today" : "Day clear"
                    detail: Backend.logged_in ? calendarAgenda.headline : "Sign in to pull calendar events"
                    model: calendarModel
                    emptyText: "No events loaded yet."
                    loading: Backend.calendar_loading
                    onOpen: root.navigate(1)
                    onRefresh: Backend.refresh_calendar()
                }

                PreviewCard {
                    theme: root.theme
                    Layout.preferredWidth: homeGrid.cardWidth
                    Layout.preferredHeight: 248
                    title: "Weather"
                    metric: weatherLocationCount + " favorites"
                    detail: "Current conditions and Doppler shortcuts"
                    model: weatherModel
                    emptyText: "Add up to 5 locations in Settings."
                    loading: Backend.weather_loading
                    onOpen: root.navigate(2)
                    onRefresh: Backend.refresh_weather()
                }

                PreviewCard {
                    theme: root.theme
                    Layout.preferredWidth: homeGrid.cardWidth
                    Layout.preferredHeight: 248
                    title: "Markets"
                    metric: stockSymbolCount + " symbols"
                    detail: "Markets, commodities, and securities"
                    model: stocksModel
                    emptyText: "Your watchlist will preview here."
                    loading: Backend.stocks_loading
                    onOpen: root.navigate(3)
                    onRefresh: Backend.refresh_stocks()
                }

                PreviewCard {
                    theme: root.theme
                    Layout.preferredWidth: homeGrid.cardWidth
                    Layout.preferredHeight: 248
                    title: "News"
                    metric: newsKeywordCount + " keywords"
                    detail: "Filtered headlines that match your terms"
                    model: newsModel
                    emptyText: "Refresh to load current headlines."
                    loading: Backend.news_loading
                    onOpen: root.navigate(4)
                    onRefresh: Backend.refresh_news()
                }

                PreviewCard {
                    theme: root.theme
                    Layout.preferredWidth: homeGrid.cardWidth
                    Layout.preferredHeight: 248
                    title: "Bluetooth Devices"
                    metric: Backend.bluetooth_connected_device ? "Connected" : "Ready to scan"
                    detail: "Native BLE control for Happy Wakey alarm hardware"
                    model: bluetoothPreviewModel
                    emptyText: "Open Devices to scan for compatible peripherals."
                    onOpen: root.navigate(7)
                    onRefresh: Backend.scan_bluetooth()
                }

                Rectangle {
                    Layout.preferredWidth: homeGrid.cardWidth
                    Layout.preferredHeight: 248
                    color: theme.surface
                    radius: 6
                    border.color: theme.border
                    border.width: 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        Text {
                            text: "Setup"
                            font.pixelSize: 16
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            Layout.fillWidth: true
                            text: "Config sync, onboarding state, API keys, and backup repo."
                            color: theme.muted
                            font.pixelSize: 12
                            wrapMode: Text.WordWrap
                        }

                        Text {
                            Layout.fillWidth: true
                            text: cfg.git_repo_path ? cfg.git_repo_path : "No git backup path yet"
                            color: theme.faint
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }

                        Item { Layout.fillHeight: true }

                        Button {
                            text: "Open Settings"
                            Layout.alignment: Qt.AlignRight
                            onClicked: root.navigate(9)
                        }
                    }
                }
            }
        }
    }

    component PreviewCard: Rectangle {
        id: card

        property string title: ""
        property string metric: ""
        property string detail: ""
        property string emptyText: ""
        property bool loading: false
        property var theme
        property alias model: previewRepeater.model
        signal open()
        signal refresh()

        color: theme.surface
        radius: 6
        border.color: theme.border
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    Layout.fillWidth: true
                    text: card.title
                    font.pixelSize: 16
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: card.metric
                    font.pixelSize: 11
                    color: theme.muted
                }

                BusyIndicator {
                    running: card.loading
                    visible: running
                    Layout.preferredWidth: 20
                    Layout.preferredHeight: 20
                }
            }

            Text {
                Layout.fillWidth: true
                text: card.detail
                color: theme.muted
                font.pixelSize: 12
                wrapMode: Text.WordWrap
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 6

                Repeater {
                    id: previewRepeater

                    delegate: Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 34
                        color: theme.surfaceAlt
                        radius: 4

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            spacing: 8

                            Text {
                                Layout.fillWidth: true
                                text: model.title
                                color: theme.text
                                font.pixelSize: 12
                                elide: Text.ElideRight
                            }

                            Text {
                                text: model.meta
                                color: theme.muted
                                font.pixelSize: 11
                                elide: Text.ElideRight
                                Layout.maximumWidth: 130
                            }
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    visible: previewRepeater.count === 0
                    text: card.emptyText
                    color: theme.muted
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Item { Layout.fillWidth: true }
                Button {
                    text: card.loading ? "Refreshing..." : "Refresh"
                    flat: true
                    enabled: !card.loading
                    onClicked: card.refresh()
                }
                Button {
                    text: "Open"
                    onClicked: card.open()
                }
            }
        }
    }
}
