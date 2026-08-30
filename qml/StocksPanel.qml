import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    color: "transparent"
    property var theme

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            Text {
                text: "📈 Markets"
                font.pixelSize: 22
                font.bold: true
                color: theme.text
            }
            Label {
                text: Backend.stocks_loading ? "Refreshing..." : ""
                color: theme.muted
                font.pixelSize: 12
            }
            Item { Layout.fillWidth: true }
            Button {
                text: Backend.stocks_loading ? "Refreshing..." : "Refresh all"
                enabled: !Backend.stocks_loading
                onClicked: Backend.refresh_stocks()
            }
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            GridLayout {
                columns: 4
                columnSpacing: 8
                rowSpacing: 8
                width: parent ? parent.width : 0

                Repeater {
                    model: stocksModel

                    Rectangle {
                        Layout.preferredWidth: 220
                        Layout.preferredHeight: 80
                        color: theme.surface
                        radius: 6
                        border.color: {
                            var ch = parseFloat(model.change)
                            return ch >= 0 ? theme.positive : theme.negative
                        }
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 8

                            ColumnLayout {
                                spacing: 2
                                Text {
                                    text: model.symbol
                                    font.pixelSize: 18
                                    font.bold: true
                                    color: theme.text
                                }
                                Text {
                                    text: model.name.length > 20 ? model.name.substring(0, 20) + "…" : model.name
                                    font.pixelSize: 10
                                    color: theme.muted
                                    elide: Text.ElideRight
                                }
                            }

                            Item { Layout.fillWidth: true }

                            ColumnLayout {
                                spacing: 2
                                Text {
                                    text: "$" + model.price
                                    font.pixelSize: 18
                                    font.bold: true
                                    color: {
                                        var ch = parseFloat(model.change)
                                        return ch >= 0 ? theme.positive : theme.negative
                                    }
                                    horizontalAlignment: Text.AlignRight
                                }
                                Text {
                                    text: {
                                        var ch = parseFloat(model.change)
                                        var cp = parseFloat(model.change_percent)
                                        return (ch >= 0 ? "+" : "") + model.change + " (" + (cp >= 0 ? "+" : "") + model.change_percent + "%)"
                                    }
                                    font.pixelSize: 11
                                    color: {
                                        var ch = parseFloat(model.change)
                                        return ch >= 0 ? theme.positive : theme.negative
                                    }
                                    horizontalAlignment: Text.AlignRight
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ListModel { id: stocksModel }

    onVisibleChanged: {
        if (visible && stocksModel.count === 0) Backend.refresh_stocks()
    }

    Connections {
        target: Backend
        function onStocks_jsonChanged() {
            try {
                var arr = JSON.parse(Backend.stocks_json)
                stocksModel.clear()
                for (var i = 0; i < arr.length; i++) {
                    var s = arr[i]
                    stocksModel.append({
                        symbol: s.symbol,
                        name: s.name || s.symbol,
                        price: s.price.toFixed(2),
                        change: s.change.toFixed(2),
                        change_percent: s.change_percent.toFixed(2)
                    })
                }
            } catch(e) {}
        }
    }
}
