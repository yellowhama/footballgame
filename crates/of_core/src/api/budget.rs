use std::time::{Duration, Instant};

/// Simulation budget to prevent infinite loops and timeouts
/// Tracks progress and allows cooperative cancellation
#[derive(Debug, Clone)]
pub struct SimBudget {
    start_time: Instant,
    max_wall_ms: u64,  // Maximum wall clock time in milliseconds
    max_minutes: u16,  // Maximum simulation minutes
    max_events: usize, // Maximum number of events

    // Progress tracking
    minutes_done: u16,
    events_done: usize,
}

impl Default for SimBudget {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            max_wall_ms: 50,  // 50ms max
            max_minutes: 120, // 120 minutes max (extra time)
            max_events: 500,  // 500 events max
            minutes_done: 0,
            events_done: 0,
        }
    }
}

impl SimBudget {
    /// Create a new budget with specified limits
    pub fn new(max_wall_ms: u64, max_minutes: u16, max_events: usize) -> Self {
        Self {
            start_time: Instant::now(),
            max_wall_ms,
            max_minutes,
            max_events,
            minutes_done: 0,
            events_done: 0,
        }
    }

    /// Create a strict budget for quick simulations
    pub fn strict() -> Self {
        Self::new(20, 95, 200)
    }

    /// Create a relaxed budget for full simulations
    pub fn relaxed() -> Self {
        Self::new(100, 150, 1000)
    }

    /// Reset the budget timer (for reuse)
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.minutes_done = 0;
        self.events_done = 0;
    }

    /// Increment minute counter and check if we should continue
    #[inline]
    pub fn tick_minute(&mut self) -> bool {
        self.minutes_done += 1;
        !self.is_exceeded()
    }

    /// Increment event counter and check if we should continue
    #[inline]
    pub fn tick_event(&mut self) -> bool {
        self.events_done += 1;
        !self.is_exceeded()
    }

    /// Check if any budget limit has been exceeded
    #[inline]
    pub fn is_exceeded(&self) -> bool {
        self.is_timeout() || self.is_minute_overflow() || self.is_event_overflow()
    }

    /// Check if wall clock time exceeded
    #[inline]
    pub fn is_timeout(&self) -> bool {
        self.start_time.elapsed() > Duration::from_millis(self.max_wall_ms)
    }

    /// Check if minute limit exceeded
    #[inline]
    pub fn is_minute_overflow(&self) -> bool {
        self.minutes_done > self.max_minutes
    }

    /// Check if event limit exceeded
    #[inline]
    pub fn is_event_overflow(&self) -> bool {
        self.events_done > self.max_events
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Get progress information
    pub fn get_progress(&self) -> (u16, usize, u64) {
        (self.minutes_done, self.events_done, self.elapsed_ms())
    }

    /// Get reason for budget exceeded (if any)
    pub fn get_exceeded_reason(&self) -> Option<String> {
        if self.is_timeout() {
            Some(format!("Wall clock timeout: {}ms > {}ms", self.elapsed_ms(), self.max_wall_ms))
        } else if self.is_minute_overflow() {
            Some(format!("Minute overflow: {} > {}", self.minutes_done, self.max_minutes))
        } else if self.is_event_overflow() {
            Some(format!("Event overflow: {} > {}", self.events_done, self.max_events))
        } else {
            None
        }
    }
}
