import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    color: "transparent"
    property var theme
    property var cfg: ({ tasks: [] })
    property string draft: ""

    function refreshConfig() {
        try {
            cfg = JSON.parse(Backend.app_config_json)
        } catch (e) {
            cfg = { tasks: [] }
        }
        if (!cfg.tasks)
            cfg.tasks = []
    }

    function persist() {
        Backend.save_config(JSON.stringify(cfg))
    }

    function addTask() {
        var title = draft.trim()
        if (!title)
            return
        var tasks = cfg.tasks.slice()
        tasks.push({
            id: "task-" + Date.now(),
            title: title.substring(0, 160),
            completed: false
        })
        cfg.tasks = tasks
        draft = ""
        persist()
    }

    Component.onCompleted: refreshConfig()

    Connections {
        target: Backend
        function onApp_config_jsonChanged() { refreshConfig() }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        Text {
            text: "✓ Daily planner"
            font.pixelSize: 22
            font.bold: true
            color: theme.text
        }
        Text {
            Layout.fillWidth: true
            text: "A focused local-first list—small enough to finish, durable enough to survive a restart."
            wrapMode: Text.WordWrap
            color: theme.muted
            font.pixelSize: 13
        }

        RowLayout {
            Layout.fillWidth: true
            TextField {
                Layout.fillWidth: true
                placeholderText: "What matters today?"
                text: draft
                maximumLength: 160
                onTextChanged: draft = text
                onAccepted: addTask()
            }
            Button {
                text: "Add"
                onClicked: addTask()
            }
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            ColumnLayout {
                width: parent ? parent.width : 0
                spacing: 8

                Repeater {
                    model: cfg.tasks || []

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 52
                        color: theme.surface
                        radius: 6
                        border.color: theme.border
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 10

                            CheckBox {
                                checked: modelData.completed === true
                                onToggled: {
                                    var tasks = cfg.tasks.slice()
                                    tasks[index] = {
                                        id: modelData.id,
                                        title: modelData.title,
                                        completed: checked
                                    }
                                    cfg.tasks = tasks
                                    persist()
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: modelData.title || ""
                                color: theme.text
                                elide: Text.ElideRight
                                font.strikeout: modelData.completed === true
                            }
                            Button {
                                text: "Remove"
                                onClicked: {
                                    var tasks = cfg.tasks.slice()
                                    tasks.splice(index, 1)
                                    cfg.tasks = tasks
                                    persist()
                                }
                            }
                        }
                    }
                }

                Text {
                    visible: !(cfg.tasks && cfg.tasks.length)
                    text: "Add one concrete task to begin your day."
                    color: theme.muted
                }
            }
        }
    }
}
