/// Canonical desktop destinations shared with Flutter and the e2e contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Destination {
    pub id: &'static str,
    pub label: &'static str,
    pub panel: usize,
}

pub const DESTINATIONS: &[Destination] = &[
    Destination {
        id: "home",
        label: "Home",
        panel: 0,
    },
    Destination {
        id: "calendar",
        label: "Calendar",
        panel: 1,
    },
    Destination {
        id: "weather",
        label: "Weather",
        panel: 2,
    },
    Destination {
        id: "markets",
        label: "Stocks",
        panel: 3,
    },
    Destination {
        id: "news",
        label: "News",
        panel: 4,
    },
    Destination {
        id: "planner",
        label: "Planner",
        panel: 5,
    },
    Destination {
        id: "focus",
        label: "Focus",
        panel: 6,
    },
    Destination {
        id: "devices",
        label: "Devices",
        panel: 7,
    },
    Destination {
        id: "settings",
        label: "Settings",
        panel: 8,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_order_matches_flutter_and_qml_sidebar() {
        let ids: Vec<_> = DESTINATIONS
            .iter()
            .map(|destination| destination.id)
            .collect();
        assert_eq!(
            ids,
            [
                "home", "calendar", "weather", "markets", "news", "planner", "focus", "devices",
                "settings"
            ]
        );
        let qml = include_str!("../qml/MainWindow.qml");
        for destination in DESTINATIONS {
            let needle = format!("label: \"{}\"", destination.label);
            assert!(
                qml.contains(&needle),
                "MainWindow.qml missing {}",
                destination.id
            );
        }
        assert!(qml.contains("PlannerPanel"));
        assert!(qml.contains("FocusPanel"));
        assert!(qml.contains("DevicesPanel"));
        let planner = include_str!("../qml/PlannerPanel.qml");
        let focus = include_str!("../qml/FocusPanel.qml");
        let devices = include_str!("../qml/DevicesPanel.qml");
        assert!(planner.contains("Daily planner"));
        assert!(focus.contains("Start focus"));
        assert!(focus.contains("Pause"));
        assert!(devices.contains("Preview alarm") || devices.contains("scan"));
        assert!(!devices.contains("token"));
        assert!(!devices.contains("owner_id"));
    }
}
