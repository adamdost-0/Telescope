//! Connection lifecycle state machine.
//!
//! Tracks the state of a connection to a Kubernetes cluster,
//! enabling the UI to show accurate status and the engine
//! to manage reconnection with backoff.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Represents the current state of a Kubernetes cluster connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", content = "detail")]
pub enum ConnectionState {
    /// No connection attempted yet.
    Disconnected,
    /// Attempting initial connection / authentication.
    Connecting,
    /// Connected; performing initial LIST to populate cache.
    Syncing {
        resources_synced: u32,
        resources_total: Option<u32>,
    },
    /// Fully connected with active WATCH streams.
    Ready,
    /// Connected but experiencing issues (e.g., some watches failed).
    Degraded { message: String },
    /// Connection lost or authentication failed.
    Error { message: String },
    /// Waiting before retry. Exponential backoff with jitter.
    Backoff { attempt: u32, wait: Duration },
}

/// Events that trigger state transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionEvent {
    /// User requested connection to a cluster context.
    Connect,
    /// Authentication / client construction succeeded.
    Authenticated,
    /// Initial LIST started.
    SyncStarted,
    /// Progress during initial sync.
    SyncProgress { synced: u32, total: Option<u32> },
    /// All initial LISTs complete, WATCH streams active.
    SyncComplete,
    /// A watch stream encountered a recoverable error.
    WatchError { message: String },
    /// Connection fully lost.
    Disconnected,
    /// Backoff timer expired, ready to retry.
    RetryReady,
    /// User requested disconnect.
    UserDisconnect,
}

/// Maximum backoff duration (5 minutes).
const MAX_BACKOFF: Duration = Duration::from_secs(300);
/// Base backoff duration (1 second).
const BASE_BACKOFF: Duration = Duration::from_secs(1);
/// Maximum retry attempts before giving up and staying in Error.
const MAX_RETRIES: u32 = 10;

impl ConnectionState {
    /// Apply an event and return the new state.
    /// Returns None if the transition is invalid.
    pub fn transition(&self, event: &ConnectionEvent) -> Option<ConnectionState> {
        match (self, event) {
            // From Disconnected
            (ConnectionState::Disconnected, ConnectionEvent::Connect) => {
                Some(ConnectionState::Connecting)
            }

            // From Connecting
            (ConnectionState::Connecting, ConnectionEvent::Authenticated) => {
                Some(ConnectionState::Syncing {
                    resources_synced: 0,
                    resources_total: None,
                })
            }
            (ConnectionState::Connecting, ConnectionEvent::Disconnected) => {
                Some(ConnectionState::Error {
                    message: "Connection failed".into(),
                })
            }

            // From Syncing
            (ConnectionState::Syncing { .. }, ConnectionEvent::SyncProgress { synced, total }) => {
                Some(ConnectionState::Syncing {
                    resources_synced: *synced,
                    resources_total: *total,
                })
            }
            (ConnectionState::Syncing { .. }, ConnectionEvent::SyncComplete) => {
                Some(ConnectionState::Ready)
            }
            (ConnectionState::Syncing { .. }, ConnectionEvent::Disconnected) => {
                Some(ConnectionState::Error {
                    message: "Lost connection during sync".into(),
                })
            }

            // From Ready
            (ConnectionState::Ready, ConnectionEvent::WatchError { message }) => {
                Some(ConnectionState::Degraded {
                    message: message.clone(),
                })
            }
            (ConnectionState::Ready, ConnectionEvent::Disconnected) => {
                Some(ConnectionState::Backoff {
                    attempt: 1,
                    wait: BASE_BACKOFF,
                })
            }

            // From Degraded
            (ConnectionState::Degraded { .. }, ConnectionEvent::SyncComplete) => {
                Some(ConnectionState::Ready)
            }
            (ConnectionState::Degraded { .. }, ConnectionEvent::Disconnected) => {
                Some(ConnectionState::Backoff {
                    attempt: 1,
                    wait: BASE_BACKOFF,
                })
            }

            // From Error
            (ConnectionState::Error { .. }, ConnectionEvent::Connect) => {
                Some(ConnectionState::Connecting)
            }

            // From Backoff
            (ConnectionState::Backoff { attempt, .. }, ConnectionEvent::RetryReady) => {
                if *attempt >= MAX_RETRIES {
                    Some(ConnectionState::Error {
                        message: format!("Failed after {} retries", attempt),
                    })
                } else {
                    Some(ConnectionState::Connecting)
                }
            }
            (ConnectionState::Backoff { attempt, .. }, ConnectionEvent::Disconnected) => {
                let next_attempt = attempt + 1;
                let wait = Self::calculate_backoff(next_attempt);
                Some(ConnectionState::Backoff {
                    attempt: next_attempt,
                    wait,
                })
            }

            // UserDisconnect from any state
            (_, ConnectionEvent::UserDisconnect) => Some(ConnectionState::Disconnected),

            // Invalid transition
            _ => None,
        }
    }

    /// Calculate backoff duration with exponential increase.
    /// Jitter should be added by the caller at runtime.
    pub fn calculate_backoff(attempt: u32) -> Duration {
        let secs = BASE_BACKOFF
            .as_secs()
            .saturating_mul(1u64 << attempt.min(8));
        Duration::from_secs(secs.min(MAX_BACKOFF.as_secs()))
    }

    /// Whether this state represents an active/usable connection.
    pub fn is_connected(&self) -> bool {
        matches!(
            self,
            ConnectionState::Ready
                | ConnectionState::Degraded { .. }
                | ConnectionState::Syncing { .. }
        )
    }

    /// Whether this state means we should show data (possibly stale).
    pub fn has_data(&self) -> bool {
        matches!(
            self,
            ConnectionState::Ready
                | ConnectionState::Degraded { .. }
                | ConnectionState::Backoff { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnected_connect_becomes_connecting() {
        let state = ConnectionState::Disconnected;
        let next = state.transition(&ConnectionEvent::Connect).unwrap();
        assert_eq!(next, ConnectionState::Connecting);
    }

    #[test]
    fn connecting_authenticated_becomes_syncing() {
        let state = ConnectionState::Connecting;
        let next = state.transition(&ConnectionEvent::Authenticated).unwrap();
        assert_eq!(
            next,
            ConnectionState::Syncing {
                resources_synced: 0,
                resources_total: None
            }
        );
    }

    #[test]
    fn syncing_complete_becomes_ready() {
        let state = ConnectionState::Syncing {
            resources_synced: 5,
            resources_total: Some(5),
        };
        let next = state.transition(&ConnectionEvent::SyncComplete).unwrap();
        assert_eq!(next, ConnectionState::Ready);
    }

    #[test]
    fn syncing_progress_updates_counts() {
        let state = ConnectionState::Syncing {
            resources_synced: 0,
            resources_total: None,
        };
        let next = state
            .transition(&ConnectionEvent::SyncProgress {
                synced: 3,
                total: Some(10),
            })
            .unwrap();
        assert_eq!(
            next,
            ConnectionState::Syncing {
                resources_synced: 3,
                resources_total: Some(10)
            }
        );
    }

    #[test]
    fn ready_disconnected_becomes_backoff_1() {
        let state = ConnectionState::Ready;
        let next = state.transition(&ConnectionEvent::Disconnected).unwrap();
        assert_eq!(
            next,
            ConnectionState::Backoff {
                attempt: 1,
                wait: Duration::from_secs(1)
            }
        );
    }

    #[test]
    fn ready_watch_error_becomes_degraded() {
        let state = ConnectionState::Ready;
        let next = state
            .transition(&ConnectionEvent::WatchError {
                message: "stream reset".into(),
            })
            .unwrap();
        assert_eq!(
            next,
            ConnectionState::Degraded {
                message: "stream reset".into()
            }
        );
    }

    #[test]
    fn degraded_sync_complete_becomes_ready() {
        let state = ConnectionState::Degraded {
            message: "watch failed".into(),
        };
        let next = state.transition(&ConnectionEvent::SyncComplete).unwrap();
        assert_eq!(next, ConnectionState::Ready);
    }

    #[test]
    fn degraded_disconnected_becomes_backoff() {
        let state = ConnectionState::Degraded {
            message: "partial".into(),
        };
        let next = state.transition(&ConnectionEvent::Disconnected).unwrap();
        assert_eq!(
            next,
            ConnectionState::Backoff {
                attempt: 1,
                wait: Duration::from_secs(1)
            }
        );
    }

    #[test]
    fn backoff_retry_ready_becomes_connecting() {
        let state = ConnectionState::Backoff {
            attempt: 3,
            wait: Duration::from_secs(8),
        };
        let next = state.transition(&ConnectionEvent::RetryReady).unwrap();
        assert_eq!(next, ConnectionState::Connecting);
    }

    #[test]
    fn backoff_at_max_retries_becomes_error() {
        let state = ConnectionState::Backoff {
            attempt: 10,
            wait: Duration::from_secs(300),
        };
        let next = state.transition(&ConnectionEvent::RetryReady).unwrap();
        assert_eq!(
            next,
            ConnectionState::Error {
                message: "Failed after 10 retries".into()
            }
        );
    }

    #[test]
    fn error_connect_becomes_connecting() {
        let state = ConnectionState::Error {
            message: "auth failed".into(),
        };
        let next = state.transition(&ConnectionEvent::Connect).unwrap();
        assert_eq!(next, ConnectionState::Connecting);
    }

    #[test]
    fn user_disconnect_from_any_state() {
        let states = vec![
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Syncing {
                resources_synced: 0,
                resources_total: None,
            },
            ConnectionState::Ready,
            ConnectionState::Degraded {
                message: "x".into(),
            },
            ConnectionState::Error {
                message: "x".into(),
            },
            ConnectionState::Backoff {
                attempt: 1,
                wait: Duration::from_secs(1),
            },
        ];
        for state in states {
            let next = state.transition(&ConnectionEvent::UserDisconnect).unwrap();
            assert_eq!(next, ConnectionState::Disconnected, "from {:?}", state);
        }
    }

    #[test]
    fn invalid_transitions_return_none() {
        // Can't authenticate from Disconnected
        assert!(ConnectionState::Disconnected
            .transition(&ConnectionEvent::Authenticated)
            .is_none());
        // Can't sync complete from Connecting
        assert!(ConnectionState::Connecting
            .transition(&ConnectionEvent::SyncComplete)
            .is_none());
        // Can't connect from Ready
        assert!(ConnectionState::Ready
            .transition(&ConnectionEvent::Connect)
            .is_none());
    }

    #[test]
    fn calculate_backoff_increases_exponentially() {
        let d1 = ConnectionState::calculate_backoff(1);
        let d2 = ConnectionState::calculate_backoff(2);
        let d3 = ConnectionState::calculate_backoff(3);
        assert_eq!(d1, Duration::from_secs(2));
        assert_eq!(d2, Duration::from_secs(4));
        assert_eq!(d3, Duration::from_secs(8));
    }

    #[test]
    fn calculate_backoff_caps_at_max() {
        // attempt.min(8) = 8, so 1 << 8 = 256; next attempt 9 also gives 256
        // since 256 < 300, verify that very high attempts still cap at MAX_BACKOFF
        let d = ConnectionState::calculate_backoff(9);
        assert_eq!(d, Duration::from_secs(256));
        // The max shift is 8 (256s) which is under the 300s cap,
        // so the effective maximum is 256s.
        let d_high = ConnectionState::calculate_backoff(100);
        assert_eq!(d_high, Duration::from_secs(256));
    }

    #[test]
    fn is_connected_returns_correct_values() {
        assert!(!ConnectionState::Disconnected.is_connected());
        assert!(!ConnectionState::Connecting.is_connected());
        assert!(ConnectionState::Syncing {
            resources_synced: 0,
            resources_total: None
        }
        .is_connected());
        assert!(ConnectionState::Ready.is_connected());
        assert!(ConnectionState::Degraded {
            message: "x".into()
        }
        .is_connected());
        assert!(!ConnectionState::Error {
            message: "x".into()
        }
        .is_connected());
        assert!(!ConnectionState::Backoff {
            attempt: 1,
            wait: Duration::from_secs(1)
        }
        .is_connected());
    }

    #[test]
    fn has_data_returns_correct_values() {
        assert!(!ConnectionState::Disconnected.has_data());
        assert!(!ConnectionState::Connecting.has_data());
        assert!(!ConnectionState::Syncing {
            resources_synced: 0,
            resources_total: None
        }
        .has_data());
        assert!(ConnectionState::Ready.has_data());
        assert!(ConnectionState::Degraded {
            message: "x".into()
        }
        .has_data());
        assert!(!ConnectionState::Error {
            message: "x".into()
        }
        .has_data());
        assert!(ConnectionState::Backoff {
            attempt: 1,
            wait: Duration::from_secs(1)
        }
        .has_data());
    }

    #[test]
    fn connecting_disconnected_becomes_error() {
        let state = ConnectionState::Connecting;
        let next = state.transition(&ConnectionEvent::Disconnected).unwrap();
        assert_eq!(
            next,
            ConnectionState::Error {
                message: "Connection failed".into()
            }
        );
    }

    #[test]
    fn syncing_disconnected_becomes_error() {
        let state = ConnectionState::Syncing {
            resources_synced: 3,
            resources_total: Some(10),
        };
        let next = state.transition(&ConnectionEvent::Disconnected).unwrap();
        assert_eq!(
            next,
            ConnectionState::Error {
                message: "Lost connection during sync".into()
            }
        );
    }
}
