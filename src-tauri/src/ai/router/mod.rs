// Intelligent Router Module
// Routes requests to optimal AI providers based on various strategies

pub mod load_balancer;
pub mod cost_optimizer;
pub mod capability_matcher;
pub mod fallback_chain;
pub mod circuit_breaker;
pub mod router;

// Re-exports
pub use router::IntelligentRouter;
pub use load_balancer::LoadBalancer;
pub use cost_optimizer::CostOptimizer;
pub use capability_matcher::CapabilityMatcher;
pub use fallback_chain::FallbackChain;
pub use circuit_breaker::{CircuitBreaker, CircuitState};
