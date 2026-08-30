import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    id: root
    color: "transparent"
    property var theme

    function rebuildDevices() {
        deviceModel.clear()
        try {
            var devices = JSON.parse(Backend.bluetooth_devices_json || "[]")
            for (var i = 0; i < devices.length; i++) {
                deviceModel.append({
                    deviceId: devices[i].id,
                    name: devices[i].name,
                    rssi: devices[i].rssi,
                    connected: devices[i].connected
                })
            }
        } catch (error) {
            Backend.set_status("Bluetooth device response was invalid")
        }
    }

    Component.onCompleted: rebuildDevices()
    Connections {
        target: Backend
        function onBluetooth_devices_jsonChanged() { root.rebuildDevices() }
    }
    ListModel { id: deviceModel }

    ColumnLayout {
        anchors.fill: parent
        spacing: 14

        RowLayout {
            Layout.fillWidth: true
            ColumnLayout {
                Layout.fillWidth: true
                Text {
                    text: "Bluetooth Devices"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }
                Text {
                    text: "Native BLE only · no browser bridge or webview"
                    font.pixelSize: 12
                    color: theme.muted
                }
            }
            Button {
                text: Backend.bluetooth_scanning ? "Scanning..." : "Scan"
                enabled: Backend.bluetooth_supported && !Backend.bluetooth_busy
                onClicked: Backend.scan_bluetooth()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: connectionColumn.implicitHeight + 28
            color: theme.surface
            radius: 6
            border.color: theme.border
            ColumnLayout {
                id: connectionColumn
                anchors.fill: parent
                anchors.margins: 14
                Text {
                    text: Backend.bluetooth_connected_device
                        ? "Connected device: " + Backend.bluetooth_connected_device
                        : "No compatible device connected"
                    color: theme.text
                    font.bold: true
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
                RowLayout {
                    Button {
                        text: "Preview alarm"
                        enabled: Backend.bluetooth_connected_device !== "" && !Backend.bluetooth_busy
                        onClicked: Backend.test_bluetooth_alarm()
                    }
                    Button {
                        text: "Disconnect"
                        enabled: Backend.bluetooth_connected_device !== "" && !Backend.bluetooth_busy
                        onClicked: Backend.disconnect_bluetooth()
                    }
                }
            }
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: deviceModel
            spacing: 8
            clip: true
            delegate: Rectangle {
                required property string deviceId
                required property string name
                required property var rssi
                required property bool connected
                width: ListView.view.width
                height: 72
                radius: 6
                color: theme.surface
                border.color: theme.border
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    ColumnLayout {
                        Layout.fillWidth: true
                        Text { text: name; color: theme.text; font.bold: true }
                        Text {
                            text: deviceId + (rssi === null ? "" : " · " + rssi + " dBm")
                            color: theme.muted
                            font.pixelSize: 11
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }
                    }
                    Button {
                        text: connected || Backend.bluetooth_connected_device === deviceId
                            ? "Connected" : "Connect"
                        enabled: !Backend.bluetooth_busy
                            && Backend.bluetooth_connected_device !== deviceId
                        onClicked: Backend.connect_bluetooth(deviceId)
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: "Only peripherals advertising the Happy Wakey service UUID are shown. Commands are versioned, bounded to 512 bytes, and never contain authentication credentials or customer identifiers."
            color: theme.muted
            wrapMode: Text.WordWrap
            font.pixelSize: 11
        }
    }
}
