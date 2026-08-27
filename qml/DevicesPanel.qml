import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    color: "transparent"
    property var theme
    readonly property string serviceUuid: "8e0e0001-7d5a-4c3f-9c31-94e9d447fc01"
    readonly property string commandUuid: "8e0e0002-7d5a-4c3f-9c31-94e9d447fc01"
    property string previewJson: ""

    function encodePreview() {
        var payload = {
            schema: "happy-wakey.ble.preview-command.v1",
            operation_id: "018f5cc6-6d8b-7b2a-9f38-269e6a7b1f11",
            action: "preview_alarm",
            duration_ms: 3000
        }
        previewJson = JSON.stringify(payload, null, 2)
        Backend.set_status("BLE preview command encoded without credentials")
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        Text {
            text: "📟 Bluetooth devices"
            font.pixelSize: 22
            font.bold: true
            color: theme.text
        }
        Text {
            Layout.fillWidth: true
            text: "Discover only Happy Wakey BLE peripherals and send bounded, credential-free alarm commands."
            wrapMode: Text.WordWrap
            color: theme.muted
            font.pixelSize: 13
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 88
            color: theme.surface
            radius: 6
            border.color: theme.border
            border.width: 1
            Text {
                anchors.fill: parent
                anchors.margins: 16
                wrapMode: Text.WordWrap
                color: theme.muted
                text: "Radio scan uses the same service UUID as Flutter. This Qt build encodes and validates the command locally; adapter scan remains best-effort until a BLE radio is present."
            }
        }

        RowLayout {
            Button {
                text: "Encode 3-second preview"
                onClicked: encodePreview()
            }
            Text {
                text: "Service " + serviceUuid
                color: theme.faint
                font.pixelSize: 11
                elide: Text.ElideMiddle
                Layout.fillWidth: true
            }
        }

        TextArea {
            Layout.fillWidth: true
            Layout.fillHeight: true
            readOnly: true
            wrapMode: TextEdit.Wrap
            text: previewJson.length ? previewJson : "Preview payload appears here. It contains schema, operation UUID, action, and duration only."
            color: theme.text
        }

        Text {
            text: "Command characteristic " + commandUuid
            color: theme.faint
            font.pixelSize: 11
        }
    }
}
