import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    id: root
    color: "transparent"

    property var theme

    function rebuildBookmarks() {
        bookmarkModel.clear()
        var config = ({})
        try {
            config = Backend.app_config_json ? JSON.parse(Backend.app_config_json) : ({})
        } catch (error) {
            config = ({})
        }
        var bookmarks = config.browser_bookmarks || []
        for (var index = 0; index < bookmarks.length; index++) {
            var bookmark = bookmarks[index]
            if (!bookmark.url) continue
            bookmarkModel.append({
                title: bookmark.title || bookmark.url,
                url: bookmark.url
            })
        }
    }

    Component.onCompleted: rebuildBookmarks()

    Connections {
        target: Backend
        function onApp_config_jsonChanged() { root.rebuildBookmarks() }
    }

    ListModel { id: bookmarkModel }

    ColumnLayout {
        anchors.fill: parent
        spacing: 14

        Text {
            text: "Browser"
            font.pixelSize: 22
            font.bold: true
            color: theme.text
        }

        Text {
            Layout.fillWidth: true
            text: "Open trusted destinations in the system browser. Embedded credentials, unsafe schemes, public IP literals, and cleartext internet URLs fail closed."
            color: theme.muted
            font.pixelSize: 13
            wrapMode: Text.WordWrap
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 58
            color: theme.accentSoft
            radius: 6
            border.color: theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10

                Text { text: "🔒"; font.pixelSize: 20 }
                Text {
                    Layout.fillWidth: true
                    text: "External-by-default browsing keeps the platform browser's sandbox, passwords, and privacy controls authoritative."
                    color: theme.text
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }
            }
        }

        ListView {
            id: bookmarkList
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 8
            clip: true
            model: bookmarkModel

            delegate: Rectangle {
                width: bookmarkList.width
                height: 72
                color: theme.surface
                radius: 6
                border.color: theme.border

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 12

                    Text { text: "◎"; font.pixelSize: 20; color: theme.accent }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            Layout.fillWidth: true
                            text: model.title
                            color: theme.text
                            font.bold: true
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: model.url
                            color: theme.muted
                            font.pixelSize: 11
                            elide: Text.ElideMiddle
                        }
                    }
                    Button {
                        text: "Open"
                        onClicked: Backend.open_url(model.url)
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                visible: bookmarkList.count === 0
                text: "No trusted links yet. Add HTTPS bookmarks in Settings."
                color: theme.muted
            }
        }
    }
}
