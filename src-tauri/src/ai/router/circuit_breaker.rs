// Circuit Breaker
// Prevents cascading failures by stopping requests to failing providers

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are blocked
    Open,
    /// Circuit is half-open, testing if provider has recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Number of successes before closing circuit (in half-open state)
    pub success_threshold: u32,
    /// Timeout before attempting to close circuit (in half-open state)
    pub timeout: Duration,
    /// Duration to keep circuit open before attempting recovery
    pub open_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            open_duration: Duration::from_secs(60),
        }
    }
}

/// Circuit breaker for provider resilience
pub struct CircuitBreaker {
    /// Circuit state
    state: Arc<RwLock<CircuitState>>,
    /// Failure count
    failure_count: Arc<RwLock<u32>>,
    /// Success count (in half-open state)
    success_count: Arc<RwLock<u32>>,
    /// Last failure time
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    /// Configuration
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Check if circuit allows requests
    pub async fn allow_request(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                let last_failure = *self.last_failure_time.read().await;
                if let Some(failure_time) = last_failure {
                    if failure_time.elapsed() >= self.config.open_duration {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    // Close circuit
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                *self.state.write().await = CircuitState::Closed;
                *self.failure_count.write().await = 0;
            }
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;

                if *failure_count >= self.config.failure_threshold {
                    // Open circuit
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure_time.write().await = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                // Immediately open circuit on failure
                *self.state.write().await = CircuitState::Open;
                *self.last_failure_time.write().await = Some(Instant::now());
                *self.success_count.write().await = 0;
            }
            CircuitState::Open => {
                // Already open, just update failure time
                *self.last_failure_time.write().await = Some(Instant::now());
            }
        }
    }

    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get failure count
    pub async fn failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }

    /// Get success count (in half-open state)
    pub async fn success_count(&self) -> u32 {
        *self.success_count.read().await
    }

    /// Reset circuit to closed state
    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        *self.last_failure_time.write().await = None;
    }

    /// Get configuration
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            failure_count: Arc::clone(&self.failure_count),
            success_count: Arc::clone(&self.success_count),
            last_failure_time: Arc::clone(&self.last_failure_time),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            open_duration: Duration::from_secs(60),
        };
        let cb = CircuitBreaker::new(config);

        // Record failures to open circuit
        for _ in 0..3 {
            cb.record_failure().await;
        }

        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            open_duration: Duration::from_millis(100), // Short for testing
        };
        let cb = CircuitBreaker::new(config);

        // Open circuit
        for _ in 0..3 {
            cb.record_failure().await;
        }

        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for open duration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(cb.allow_request().await);
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_close() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            open_duration: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        // Open circuit
        for _ in 0..3 {
            cb.record_failure().await;
        }

        // Wait for open duration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Record successes to close circuit
        cb.record_success().await;
        cb.record_success().await;

        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::default();

        // Open circuit
        for _ in 0..5 {
            cb.record_failure().await;
        }

        assert_eq!(cb.state().await, CircuitState::Open);

        // Reset
        cb.reset().await;

        assert_eq!(cb.state().await, CircuitState::Closed);
        assert_eq!(cb.failure_count().await, 0);
    }
}
