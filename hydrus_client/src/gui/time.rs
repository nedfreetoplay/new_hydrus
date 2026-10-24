//! Time and subscription display helpers from `ClientGUITime.py` and
//! `ClientGUISubscriptions.py`.

use hydrus_core::hydrus_time;
use serde_json::Value;

use crate::importing::Subscription;

pub fn pretty_timestamp(timestamp: Option<i64>) -> String {
    hydrus_time::timestamp_to_pretty_time(timestamp.map(|value| value as f64), false, true)
}

pub fn pretty_duration_ms(duration_ms: Option<i64>) -> String {
    match duration_ms {
        Some(duration_ms) => {
            hydrus_time::milliseconds_duration_to_pretty_time(Some(duration_ms), true)
        }
        None => "no duration".to_string(),
    }
}

pub fn pretty_time_delta(seconds: i64) -> String {
    hydrus_time::time_delta_to_pretty_timedelta(Some(seconds as f64), true, false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionPanelRow {
    pub name: String,
    pub query: String,
    pub paused: bool,
    pub next_work_time: i64,
}

impl SubscriptionPanelRow {
    pub fn from_subscription(subscription: &Subscription) -> Self {
        Self {
            name: subscription.get_name().to_string(),
            query: subscription.get_query().to_string(),
            paused: subscription.paused,
            next_work_time: subscription.get_best_earliest_next_work_time(),
        }
    }

    pub fn to_api_dict(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "query": self.query,
            "paused": self.paused,
            "next_work_time": self.next_work_time,
        })
    }
}

#[derive(Debug, Default)]
pub struct SubscriptionsPanelState {
    pub rows: Vec<SubscriptionPanelRow>,
}

impl SubscriptionsPanelState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_subscriptions(&mut self, subscriptions: impl IntoIterator<Item = Subscription>) {
        self.rows = subscriptions
            .into_iter()
            .map(|subscription| SubscriptionPanelRow::from_subscription(&subscription))
            .collect();
    }

    pub fn add_row(&mut self, row: SubscriptionPanelRow) {
        self.rows.push(row);
    }

    pub fn remove_row(&mut self, name: &str) -> bool {
        let previous = self.rows.len();
        self.rows.retain(|row| row.name != name);
        previous != self.rows.len()
    }

    pub fn toggle_pause(&mut self, name: &str) -> bool {
        if let Some(row) = self.rows.iter_mut().find(|row| row.name == name) {
            row.paused = !row.paused;
            true
        } else {
            false
        }
    }

    pub fn get_due_rows(&self, now: i64) -> Vec<&SubscriptionPanelRow> {
        self.rows
            .iter()
            .filter(|row| !row.paused && now >= row.next_work_time)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_helpers_handle_missing_values() {
        assert_eq!(pretty_timestamp(None), "unknown time");
        assert_eq!(pretty_duration_ms(None), "no duration");
        assert!(!pretty_time_delta(60).is_empty());
    }

    #[test]
    fn subscription_rows_convert_from_models() {
        let subscription = Subscription::new("sub", "query");
        let row = SubscriptionPanelRow::from_subscription(&subscription);

        assert_eq!(row.name, "sub");
        assert_eq!(row.query, "query");
        assert_eq!(row.to_api_dict()["name"], "sub");
    }

    #[test]
    fn panel_state_can_pause_and_toggle() {
        let mut state = SubscriptionsPanelState::new();
        state.set_subscriptions([Subscription::new("sub", "query")]);

        assert!(state.toggle_pause("sub"));
        assert!(state.rows[0].paused);
        assert!(state.get_due_rows(i64::MAX).is_empty());

        assert!(state.toggle_pause("sub"));
        assert!(!state.rows[0].paused);
        assert_eq!(state.get_due_rows(i64::MAX).len(), 1);
    }

    #[test]
    fn rows_can_be_removed_by_name() {
        let mut state = SubscriptionsPanelState::new();
        state.add_row(SubscriptionPanelRow {
            name: "sub".to_string(),
            query: "q".to_string(),
            paused: false,
            next_work_time: 0,
        });

        assert!(state.remove_row("sub"));
        assert!(state.is_empty());
    }
}
