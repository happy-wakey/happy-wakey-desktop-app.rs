pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.happywakey

Rectangle {
    id: root
    color: "transparent"

    property var theme
    property var inbox: ({ state: "not_connected", unread_total: 0, items: [], note: "" })
    property var messages: ({ state: "not_connected", unread_total: null, items: [], note: "" })
    property var health: ({ state: "not_connected", sleep: null, biometrics: null, note: "" })
    property bool anyLoading: Backend.inbox_loading || Backend.messages_loading || Backend.health_loading

    function parseJson(value, fallback) {
        try {
            return value && value.length > 0 ? JSON.parse(value) : fallback
        } catch (error) {
            return fallback
        }
    }

    function stateLabel(state, loading) {
        if (loading) return "Loading"
        switch (state) {
        case "ready": return "Ready"
        case "empty": return "Nothing urgent"
        case "degraded": return "Partially available"
        case "failed": return "Unavailable"
        default: return "Not connected"
        }
    }

    function stateColor(state, loading) {
        if (loading) return theme.accent
        if (state === "ready" || state === "empty") return theme.positive
        if (state === "degraded") return theme.warning
        if (state === "failed") return theme.negative
        return theme.muted
    }

    function formatMinutes(value) {
        if (value === null || value === undefined) return "—"
        var minutes = Math.max(0, Number(value))
        var hours = Math.floor(minutes / 60)
        var remainder = Math.round(minutes % 60)
        return hours > 0 ? hours + "h " + remainder + "m" : remainder + "m"
    }

    function formatNumber(value, suffix) {
        if (value === null || value === undefined) return "—"
        return Math.round(Number(value)).toLocaleString(Qt.locale()) + (suffix || "")
    }

    function rebuildInbox() {
        inbox = parseJson(Backend.inbox_json, inbox)
        inboxModel.clear()
        var items = inbox.items || []
        for (var i = 0; i < Math.min(items.length, 20); i++) {
            var item = items[i]
            inboxModel.append({
                subject: item.subject || "No subject",
                sender: item.sender_display || item.sender_address || "Unknown sender",
                received_at: item.received_at || "",
                important: item.important === true,
                unread: item.unread === true,
                action_url: item.url || ""
            })
        }
    }

    function rebuildMessages() {
        messages = parseJson(Backend.messages_json, messages)
        messageModel.clear()
        var items = messages.items || []
        for (var i = 0; i < Math.min(items.length, 20); i++) {
            var item = items[i]
            messageModel.append({
                conversation: item.conversation || "Conversation",
                source: item.source || "message",
                preview: item.preview || "No provider-approved preview",
                received_at: item.received_at || "",
                unread_count: Number(item.unread_count || 0),
                mentioned: item.mentions_owner === true,
                action_url: item.url || ""
            })
        }
    }

    function rebuildHealth() {
        health = parseJson(Backend.health_json, health)
        anomalyModel.clear()
        var biometrics = health.biometrics || null
        var anomalies = biometrics && biometrics.anomalies ? biometrics.anomalies : []
        for (var i = 0; i < Math.min(anomalies.length, 8); i++) {
            var anomaly = anomalies[i]
            anomalyModel.append({
                metric: anomaly.metric || "Metric",
                direction: anomaly.direction || "changed",
                severity: anomaly.severity || "informational",
                observed: Number(anomaly.observed || 0),
                baseline: Number(anomaly.baseline_mean || 0)
            })
        }
    }

    function refreshAll() {
        Backend.refresh_inbox()
        Backend.refresh_messages()
        Backend.refresh_health()
    }

    Component.onCompleted: {
        rebuildInbox()
        rebuildMessages()
        rebuildHealth()
    }

    Connections {
        target: Backend
        function onInbox_jsonChanged() { root.rebuildInbox() }
        function onMessages_jsonChanged() { root.rebuildMessages() }
        function onHealth_jsonChanged() { root.rebuildHealth() }
    }

    ListModel { id: inboxModel }
    ListModel { id: messageModel }
    ListModel { id: anomalyModel }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Morning brief"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "A bounded, consented view of what deserves attention this morning"
                    font.pixelSize: 12
                    color: theme.muted
                }
            }

            BusyIndicator {
                running: root.anyLoading
                visible: running
                Layout.preferredWidth: 24
                Layout.preferredHeight: 24
            }

            Button {
                text: root.anyLoading ? "Loading..." : "Load consented sources"
                enabled: Backend.logged_in && !root.anyLoading
                highlighted: true
                onClicked: root.refreshAll()
            }
        }

        Text {
            Layout.fillWidth: true
            visible: !Backend.logged_in
            text: "Sign in to choose provider access. Nothing is loaded automatically, and one unavailable source never blocks the others."
            color: theme.warning
            font.pixelSize: 12
            wrapMode: Text.WordWrap
        }

        ScrollView {
            id: briefScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: briefScroll.availableWidth
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    SummaryCard {
                        Layout.fillWidth: true
                        theme: root.theme
                        title: "Important email"
                        metric: String(root.inbox.unread_total || 0)
                        detail: "unread in the bounded metadata window"
                        status: root.stateLabel(root.inbox.state, Backend.inbox_loading)
                        statusColor: root.stateColor(root.inbox.state, Backend.inbox_loading)
                    }

                    SummaryCard {
                        Layout.fillWidth: true
                        theme: root.theme
                        title: "Direct messages"
                        metric: root.messages.unread_total === null || root.messages.unread_total === undefined
                            ? "—" : String(root.messages.unread_total)
                        detail: "provider-approved unread threads"
                        status: root.stateLabel(root.messages.state, Backend.messages_loading)
                        statusColor: root.stateColor(root.messages.state, Backend.messages_loading)
                    }

                    SummaryCard {
                        Layout.fillWidth: true
                        theme: root.theme
                        title: "Sleep & recovery"
                        metric: root.health.sleep ? root.formatMinutes(root.health.sleep.asleep_minutes) : "—"
                        detail: root.health.biometrics && root.health.biometrics.readiness_score !== null
                            ? "Readiness " + Math.round(root.health.biometrics.readiness_score)
                            : "No readiness score reported"
                        status: root.stateLabel(root.health.state, Backend.health_loading)
                        statusColor: root.stateColor(root.health.state, Backend.health_loading)
                    }
                }

                LaneSection {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 112 + inboxModel.count * 58
                    theme: root.theme
                    title: "Important email"
                    note: root.inbox.note || ""
                    status: root.stateLabel(root.inbox.state, Backend.inbox_loading)
                    statusColor: root.stateColor(root.inbox.state, Backend.inbox_loading)
                    loading: Backend.inbox_loading
                    actionLabel: "Refresh email"
                    onRefresh: Backend.refresh_inbox()

                    Repeater {
                        model: inboxModel

                        delegate: BriefRow {
                            Layout.fillWidth: true
                            theme: root.theme
                            title: (model.important ? "★ " : "") + model.subject
                            detail: model.sender + (model.unread ? " · unread" : "")
                            meta: model.received_at
                            actionUrl: model.action_url
                        }
                    }
                }

                LaneSection {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 112 + messageModel.count * 72
                    theme: root.theme
                    title: "Direct messages"
                    note: root.messages.note || ""
                    status: root.stateLabel(root.messages.state, Backend.messages_loading)
                    statusColor: root.stateColor(root.messages.state, Backend.messages_loading)
                    loading: Backend.messages_loading
                    actionLabel: "Refresh messages"
                    onRefresh: Backend.refresh_messages()

                    Repeater {
                        model: messageModel

                        delegate: BriefRow {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 64
                            theme: root.theme
                            title: (model.mentioned ? "@ " : "") + model.conversation
                            detail: model.source + " · " + model.preview
                            meta: model.unread_count + " unread"
                            actionUrl: model.action_url
                        }
                    }
                }

                LaneSection {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 210 + anomalyModel.count * 52
                    theme: root.theme
                    title: "Sleep & recovery"
                    note: root.health.note || ""
                    status: root.stateLabel(root.health.state, Backend.health_loading)
                    statusColor: root.stateColor(root.health.state, Backend.health_loading)
                    loading: Backend.health_loading
                    actionLabel: "Refresh health"
                    onRefresh: Backend.refresh_health()

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        MetricPill {
                            Layout.fillWidth: true
                            theme: root.theme
                            label: "Asleep"
                            value: root.health.sleep ? root.formatMinutes(root.health.sleep.asleep_minutes) : "—"
                        }
                        MetricPill {
                            Layout.fillWidth: true
                            theme: root.theme
                            label: "Sleep score"
                            value: root.health.sleep && root.health.sleep.sleep_score !== null
                                ? String(Math.round(root.health.sleep.sleep_score)) : "—"
                        }
                        MetricPill {
                            Layout.fillWidth: true
                            theme: root.theme
                            label: "Steps"
                            value: root.health.biometrics
                                ? root.formatNumber(root.health.biometrics.steps, "") : "—"
                        }
                        MetricPill {
                            Layout.fillWidth: true
                            theme: root.theme
                            label: "Resting heart rate"
                            value: root.health.biometrics
                                ? root.formatNumber(root.health.biometrics.resting_heart_rate_bpm, " bpm") : "—"
                        }
                    }

                    Repeater {
                        model: anomalyModel

                        delegate: BriefRow {
                            Layout.fillWidth: true
                            theme: root.theme
                            title: model.metric + " " + model.direction
                            detail: "Observed " + model.observed.toFixed(1)
                                + " · baseline " + model.baseline.toFixed(1)
                            meta: model.severity
                            actionUrl: ""
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: "Health observations are summaries from connected devices, not medical advice. Missing measurements remain missing rather than being shown as zero."
                    color: theme.faint
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }
            }
        }
    }

    component SummaryCard: Rectangle {
        id: summary
        required property var theme
        property string title: ""
        property string metric: "—"
        property string detail: ""
        property string status: ""
        property color statusColor: theme.muted

        Layout.preferredHeight: 118
        color: theme.surface
        radius: 6
        border.color: theme.border
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 3

            RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true
                    text: summary.title
                    color: theme.muted
                    font.pixelSize: 12
                    font.bold: true
                }
                Text {
                    text: summary.status
                    color: summary.statusColor
                    font.pixelSize: 10
                    font.bold: true
                }
            }

            Text {
                text: summary.metric
                color: theme.text
                font.pixelSize: 26
                font.bold: true
            }

            Text {
                Layout.fillWidth: true
                text: summary.detail
                color: theme.muted
                font.pixelSize: 11
                elide: Text.ElideRight
            }
        }
    }

    component LaneSection: Rectangle {
        id: lane
        required property var theme
        property string title: ""
        property string note: ""
        property string status: ""
        property color statusColor: theme.muted
        property bool loading: false
        property string actionLabel: "Refresh"
        default property alias content: laneContent.data
        signal refresh()

        color: theme.surface
        radius: 6
        border.color: theme.border
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 6

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    Layout.fillWidth: true
                    text: lane.title
                    color: theme.text
                    font.pixelSize: 15
                    font.bold: true
                }

                Text {
                    text: lane.status
                    color: lane.statusColor
                    font.pixelSize: 11
                    font.bold: true
                }

                Button {
                    text: lane.loading ? "Loading..." : lane.actionLabel
                    enabled: Backend.logged_in && !lane.loading
                    flat: true
                    onClicked: lane.refresh()
                }
            }

            Text {
                Layout.fillWidth: true
                text: lane.note
                color: theme.muted
                font.pixelSize: 11
                wrapMode: Text.WordWrap
                visible: text.length > 0
            }

            ColumnLayout {
                id: laneContent
                Layout.fillWidth: true
                spacing: 4
            }
        }
    }

    component BriefRow: Rectangle {
        id: briefRow
        required property var theme
        property string title: ""
        property string detail: ""
        property string meta: ""
        property string actionUrl: ""

        Layout.preferredHeight: 52
        color: theme.surfaceAlt
        radius: 4

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    Layout.fillWidth: true
                    text: briefRow.title
                    color: theme.text
                    font.pixelSize: 12
                    font.bold: true
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: briefRow.detail
                    color: theme.muted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }

            Text {
                text: briefRow.meta
                color: theme.faint
                font.pixelSize: 10
                Layout.maximumWidth: 130
                elide: Text.ElideRight
            }

            Button {
                visible: briefRow.actionUrl.length > 0
                text: "Open"
                flat: true
                onClicked: Backend.open_url(briefRow.actionUrl)
            }
        }
    }

    component MetricPill: Rectangle {
        id: metricPill
        required property var theme
        property string label: ""
        property string value: "—"

        Layout.preferredHeight: 62
        color: theme.accentSoft
        radius: 4

        Column {
            anchors.centerIn: parent
            spacing: 2
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: metricPill.value
                color: theme.text
                font.pixelSize: 15
                font.bold: true
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: metricPill.label
                color: theme.muted
                font.pixelSize: 10
            }
        }
    }
}
