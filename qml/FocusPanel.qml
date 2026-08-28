import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    color: "transparent"
    property var theme
    property var cfg: ({ focus_minutes: 25 })
    property string phase: "idle"
    property int remainingSeconds: 25 * 60

    function focusMinutes() {
        var value = parseInt(cfg.focus_minutes, 10)
        if (isNaN(value))
            return 25
        return Math.min(120, Math.max(5, value))
    }

    function refreshConfig() {
        try {
            cfg = JSON.parse(Backend.app_config_json)
        } catch (e) {
            cfg = { focus_minutes: 25 }
        }
        if (phase === "idle")
            remainingSeconds = focusMinutes() * 60
    }

    function persistMinutes(minutes) {
        cfg.focus_minutes = minutes
        Backend.save_config(JSON.stringify(cfg))
    }

    Timer {
        id: tick
        interval: 1000
        repeat: true
        running: phase === "running"
        onTriggered: {
            if (remainingSeconds <= 1) {
                remainingSeconds = 0
                phase = "completed"
            } else {
                remainingSeconds -= 1
            }
        }
    }

    function clockLabel() {
        var total = remainingSeconds
        var minutes = Math.floor(total / 60)
        var seconds = total % 60
        return String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0")
    }

    Component.onCompleted: refreshConfig()

    Connections {
        target: Backend
        function onApp_config_jsonChanged() { refreshConfig() }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 16

        Text {
            text: "⏱ Focus"
            font.pixelSize: 22
            font.bold: true
            color: theme.text
        }
        Text {
            Layout.fillWidth: true
            text: "A second explicit state machine governs start, pause, resume, completion, and reset."
            wrapMode: Text.WordWrap
            color: theme.muted
            font.pixelSize: 13
        }

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 280
            Layout.preferredHeight: 220
            color: theme.surface
            radius: 8
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                anchors.centerIn: parent
                spacing: 8
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: clockLabel()
                    font.pixelSize: 42
                    font.bold: true
                    color: theme.text
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: phase.toUpperCase()
                    color: theme.muted
                }
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 10

            Button {
                text: "Start focus"
                visible: phase === "idle" || phase === "completed"
                onClicked: {
                    remainingSeconds = focusMinutes() * 60
                    phase = "running"
                }
            }
            Button {
                text: "Pause"
                visible: phase === "running"
                onClicked: phase = "paused"
            }
            Button {
                text: "Resume"
                visible: phase === "paused"
                onClicked: phase = "running"
            }
            Button {
                text: "Reset"
                onClicked: {
                    phase = "idle"
                    remainingSeconds = focusMinutes() * 60
                }
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            Text {
                text: "Duration (minutes)"
                color: theme.muted
            }
            SpinBox {
                from: 5
                to: 120
                value: focusMinutes()
                enabled: phase === "idle"
                onValueModified: persistMinutes(value)
            }
        }
    }
}
