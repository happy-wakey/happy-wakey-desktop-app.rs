import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    id: root
    color: "transparent"
    property var theme

    ColumnLayout {
        anchors.fill: parent
        spacing: 16

        Text {
            text: "⚙ Settings"
            font.pixelSize: 22
            font.bold: true
            color: theme.text
        }

        ScrollView {
            id: settingsScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: settingsScroll.availableWidth
                spacing: 20

                // ---- Account Section ----
                SectionBox {
                    title: "Account"
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: Backend.logged_in
                                ? "Signed in as " + Backend.user_email
                                : "Not signed in"
                            font.pixelSize: 14
                            color: theme.text
                        }

                        RowLayout {
                            spacing: 8
                            Button {
                                text: "Sign in with Google"
                                visible: !Backend.logged_in
                                enabled: !Backend.auth_busy
                                onClicked: Backend.login("google")
                            }
                            Button {
                                text: "Sign in with Apple"
                                visible: !Backend.logged_in
                                enabled: !Backend.auth_busy
                                onClicked: Backend.login("apple")
                            }
                            Button {
                                text: "Sign in with Microsoft"
                                visible: !Backend.logged_in
                                enabled: !Backend.auth_busy
                                onClicked: Backend.login("microsoft")
                            }
                            Button {
                                text: "Sign Out"
                                visible: Backend.logged_in
                                onClicked: Backend.logout()
                            }
                        }
                    }
                }

                SectionBox {
                    title: "Calendar Reminders"
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 12

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Switch {
                                id: remindersEnabled
                                text: "Desktop reminders"
                                checked: true
                            }
                            CheckBox {
                                id: reminder30
                                text: "30 min"
                                checked: true
                                enabled: remindersEnabled.checked
                            }
                            CheckBox {
                                id: reminder10
                                text: "10 min"
                                checked: true
                                enabled: remindersEnabled.checked
                            }
                            CheckBox {
                                id: reminder5
                                text: "5 min"
                                enabled: remindersEnabled.checked
                            }
                            Item { Layout.fillWidth: true }
                            Button {
                                text: "Test desktop"
                                onClicked: Backend.test_notification()
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Switch {
                                id: cloudRemindersEnabled
                                text: "Email reminders when this app is closed"
                                enabled: Backend.logged_in
                            }
                            Item { Layout.fillWidth: true }
                            Button {
                                text: "Test cloud email"
                                enabled: Backend.logged_in
                                onClicked: Backend.test_cloud_notification()
                            }
                        }
                    }
                }

                // ---- Weather Locations ----
                SectionBox {
                    title: "Weather Locations (max 5)"
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        ListView {
                            id: weatherLocList
                            Layout.fillWidth: true
                            Layout.preferredHeight: 120
                            model: weatherLocModel
                            delegate: RowLayout {
                                width: weatherLocList.width
                                spacing: 8
                                Text {
                                    text: model.name + " (" + model.lat + ", " + model.lon + ")"
                                    color: theme.text
                                    font.pixelSize: 13
                                    Layout.fillWidth: true
                                }
                                Button {
                                    text: "✕"
                                    flat: true
                                    onClicked: weatherLocModel.remove(index)
                                }
                            }
                        }

                        RowLayout {
                            spacing: 4
                            TextField {
                                id: locName
                                placeholderText: "Name"
                                Layout.preferredWidth: 100
                            }
                            TextField {
                                id: locLat
                                placeholderText: "Lat"
                                Layout.preferredWidth: 80
                                validator: DoubleValidator {}
                            }
                            TextField {
                                id: locLon
                                placeholderText: "Lon"
                                Layout.preferredWidth: 80
                                validator: DoubleValidator {}
                            }
                            Button {
                                text: "Add"
                                onClicked: {
                                    if (weatherLocModel.count >= 5) return
                                    if (locName.text && locLat.text && locLon.text) {
                                        weatherLocModel.append({
                                            name: locName.text,
                                            lat: locLat.text,
                                            lon: locLon.text
                                        })
                                        locName.text = ""
                                        locLat.text = ""
                                        locLon.text = ""
                                    }
                                }
                            }
                        }
                    }
                }

                // ---- Stock Symbols ----
                SectionBox {
                    title: "Stock Symbols (max 20)"
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        ListView {
                            id: stockList
                            Layout.fillWidth: true
                            Layout.preferredHeight: 120
                            model: stockSymbolModel
                            delegate: RowLayout {
                                width: stockList.width
                                spacing: 8
                                Text {
                                    text: model.symbol
                                    color: theme.text
                                    font.pixelSize: 13
                                    Layout.fillWidth: true
                                }
                                Button {
                                    text: "✕"
                                    flat: true
                                    onClicked: stockSymbolModel.remove(index)
                                }
                            }
                        }

                        RowLayout {
                            spacing: 4
                            TextField {
                                id: newSymbol
                                placeholderText: "Symbol (e.g. AAPL)"
                                Layout.preferredWidth: 120
                            }
                            Button {
                                text: "Add"
                                onClicked: {
                                    if (stockSymbolModel.count >= 20) return
                                    var sym = newSymbol.text.trim().toUpperCase()
                                    if (sym) {
                                        stockSymbolModel.append({ symbol: sym })
                                        newSymbol.text = ""
                                    }
                                }
                            }
                        }
                    }
                }

                // ---- News Keywords ----
                SectionBox {
                    title: "News Keywords"
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Flow {
                            Layout.fillWidth: true
                            spacing: 6
                            Repeater {
                                model: newsKeywordModel
                                Rectangle {
                                    height: 28
                                    width: keywordLabel.implicitWidth + 20
                                    color: theme.border
                                    radius: 4
                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 6
                                        anchors.rightMargin: 4
                                        spacing: 4
                                        Text {
                                            id: keywordLabel
                                            text: model.keyword
                                            color: theme.text
                                            font.pixelSize: 12
                                        }
                                        Text {
                                            text: "✕"
                                            font.pixelSize: 10
                                            color: theme.faint
                                            MouseArea {
                                                anchors.fill: parent
                                                onClicked: newsKeywordModel.remove(index)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        RowLayout {
                            spacing: 4
                            TextField {
                                id: newKeyword
                                placeholderText: "Add keyword…"
                                Layout.fillWidth: true
                            }
                            Button {
                                text: "Add"
                                onClicked: {
                                    var kw = newKeyword.text.trim()
                                    if (kw) {
                                        newsKeywordModel.append({ keyword: kw })
                                        newKeyword.text = ""
                                    }
                                }
                            }
                        }
                    }
                }

                // ---- Bookmarks (Quick Browser URLs) ----
                SectionBox {
                    title: "Browser Bookmarks"
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        ListView {
                            id: bookmarkList
                            Layout.fillWidth: true
                            Layout.preferredHeight: 100
                            model: bookmarkModel
                            delegate: RowLayout {
                                width: bookmarkList.width
                                spacing: 8
                                Text {
                                    text: model.title + " (" + model.url + ")"
                                    color: theme.text
                                    font.pixelSize: 12
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                                Button {
                                    text: "✕"
                                    flat: true
                                    onClicked: bookmarkModel.remove(index)
                                }
                            }
                        }

                        RowLayout {
                            spacing: 4
                            TextField {
                                id: bmTitle
                                placeholderText: "Title"
                                Layout.preferredWidth: 120
                            }
                            TextField {
                                id: bmUrl
                                placeholderText: "URL"
                                Layout.fillWidth: true
                            }
                            Button {
                                text: "Add"
                                onClicked: {
                                    if (bmTitle.text && bmUrl.text) {
                                        bookmarkModel.append({
                                            id: "",
                                            title: bmTitle.text,
                                            url: bmUrl.text
                                        })
                                        bmTitle.text = ""
                                        bmUrl.text = ""
                                    }
                                }
                            }
                        }
                    }
                }

                // ---- Git Backup ----
                SectionBox {
                    title: "Git Backup (Optional)"
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: "Back up your config to a private git repo."
                            font.pixelSize: 12
                            color: theme.muted
                        }

                        RowLayout {
                            spacing: 4
                            TextField {
                                id: gitRepoPath
                                placeholderText: "git@github.com:user/repo.git or /local/path"
                                Layout.fillWidth: true
                                text: {
                                    try {
                                        var cfg = JSON.parse(Backend.app_config_json)
                                        return cfg.git_repo_path || ""
                                    } catch(e) { return "" }
                                }
                            }
                            Button {
                                text: "Save & Sync"
                                onClicked: {
                                    Backend.set_status("Git sync not yet implemented in this build")
                                }
                            }
                        }
                    }
                }

                // ---- Save / Reset ----
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Item { Layout.fillWidth: true }
                    Button {
                        text: "Save Settings"
                        highlighted: true
                        onClicked: saveAllSettings()
                    }
                    Button {
                        text: "Reset to Defaults"
                        onClicked: {
                            var defaults = {
                                version: "0.1.0",
                                user_id: Backend.user_id,
                                supabase_session: null,
                                calendar_providers: [],
                                weather_locations: [],
                                stock_symbols: ["AAPL","GOOGL","MSFT","AMZN","NVDA","META","TSLA","SPY","QQQ","GLD","AMD","WMT","JPM","V","KO","DIS","NFLX","BA","XOM","PG"],
                                news_keywords: ["technology","AI","markets"],
                                browser_bookmarks: [],
                                git_repo_path: "",
                                supabase_sync_enabled: true,
                                reminder_settings: {
                                    enabled: true,
                                    cloud_email_enabled: false,
                                    offsets_minutes: [30, 10]
                                },
                                onboarding: {
                                    completed: false,
                                    current_step: "welcome",
                                    step_index: 0,
                                    updated_at: null
                                }
                            }
                            var savedCfg = JSON.stringify(defaults)
                            Backend.save_config(savedCfg)
                            Backend.reload_config()
                        }
                    }
                }

                Item { Layout.preferredHeight: 32 }
            }
        }
    }

    // Helper to collect and save all settings
    function saveAllSettings() {
        try {
            var cfg = JSON.parse(Backend.app_config_json)

            // Weather locations
            var locs = []
            for (var i = 0; i < weatherLocModel.count; i++) {
                locs.push({
                    name: weatherLocModel.get(i).name,
                    lat: parseFloat(weatherLocModel.get(i).lat),
                    lon: parseFloat(weatherLocModel.get(i).lon)
                })
            }
            cfg.weather_locations = locs

            // Stock symbols
            var syms = []
            for (var j = 0; j < stockSymbolModel.count; j++) {
                syms.push(stockSymbolModel.get(j).symbol)
            }
            cfg.stock_symbols = syms

            // News keywords
            var kws = []
            for (var k = 0; k < newsKeywordModel.count; k++) {
                kws.push(newsKeywordModel.get(k).keyword)
            }
            cfg.news_keywords = kws

            // Bookmarks
            var bms = []
            for (var m = 0; m < bookmarkModel.count; m++) {
                bms.push({
                    id: bookmarkModel.get(m).id || "",
                    title: bookmarkModel.get(m).title,
                    url: bookmarkModel.get(m).url
                })
            }
            cfg.browser_bookmarks = bms

            // Git repo
            cfg.git_repo_path = gitRepoPath.text || ""

            var reminderOffsets = []
            if (reminder30.checked) reminderOffsets.push(30)
            if (reminder10.checked) reminderOffsets.push(10)
            if (reminder5.checked) reminderOffsets.push(5)
            cfg.reminder_settings = {
                enabled: remindersEnabled.checked,
                cloud_email_enabled: cloudRemindersEnabled.checked,
                offsets_minutes: reminderOffsets
            }

            Backend.save_config(JSON.stringify(cfg))
            Backend.reload_config()
        } catch(e) {
            Backend.set_status("Save error: " + e)
        }
    }

    // ---- Models ----
    ListModel { id: weatherLocModel }
    ListModel { id: stockSymbolModel }
    ListModel { id: newsKeywordModel }
    ListModel { id: bookmarkModel }

    // Load existing config when panel becomes visible
    onVisibleChanged: {
        if (!visible) return

        try {
            var cfg = JSON.parse(Backend.app_config_json)

            var reminderSettings = cfg.reminder_settings || {
                enabled: true,
                cloud_email_enabled: false,
                offsets_minutes: [30, 10]
            }
            var offsets = reminderSettings.offsets_minutes || []
            remindersEnabled.checked = reminderSettings.enabled !== false
            cloudRemindersEnabled.checked = reminderSettings.cloud_email_enabled === true
            reminder30.checked = offsets.indexOf(30) >= 0
            reminder10.checked = offsets.indexOf(10) >= 0
            reminder5.checked = offsets.indexOf(5) >= 0

            weatherLocModel.clear()
            if (cfg.weather_locations) {
                for (var i = 0; i < cfg.weather_locations.length; i++) {
                    var w = cfg.weather_locations[i]
                    weatherLocModel.append({
                        name: w.name,
                        lat: String(w.lat),
                        lon: String(w.lon)
                    })
                }
            }

            stockSymbolModel.clear()
            if (cfg.stock_symbols) {
                for (var j = 0; j < cfg.stock_symbols.length; j++) {
                    stockSymbolModel.append({ symbol: cfg.stock_symbols[j] })
                }
            }

            newsKeywordModel.clear()
            if (cfg.news_keywords) {
                for (var k = 0; k < cfg.news_keywords.length; k++) {
                    newsKeywordModel.append({ keyword: cfg.news_keywords[k] })
                }
            }

            bookmarkModel.clear()
            if (cfg.browser_bookmarks) {
                for (var m = 0; m < cfg.browser_bookmarks.length; m++) {
                    var b = cfg.browser_bookmarks[m]
                    bookmarkModel.append({
                        id: b.id || "",
                        title: b.title || "",
                        url: b.url || ""
                    })
                }
            }
        } catch(e) {}
    }

    // ---- Section Box Component ----
    // Title sits above the content (no overlap) and height grows with content
    // (no clipping). Content is provided as the default children.
    component SectionBox: Rectangle {
        id: sectionBox
        default property alias content: contentColumn.data
        property string title: ""
        property var panelTheme: root.theme

        color: panelTheme.surface
        radius: 6
        border.color: panelTheme.border
        border.width: 1
        implicitHeight: outerColumn.implicitHeight + 28

        ColumnLayout {
            id: outerColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.leftMargin: 16
            anchors.rightMargin: 16
            anchors.topMargin: 14
            spacing: 10

            Text {
                text: sectionBox.title
                font.pixelSize: 13
                font.bold: true
                color: sectionBox.panelTheme.muted
            }

            ColumnLayout {
                id: contentColumn
                Layout.fillWidth: true
                spacing: 8
            }
        }
    }
}
