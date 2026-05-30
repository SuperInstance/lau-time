use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// GameTime
// ---------------------------------------------------------------------------

/// Core game clock. `tick` advances monotonically; `ticks_per_second` maps
/// ticks to real-world seconds. `paused` freezes advancement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTime {
    pub tick: u64,
    pub ticks_per_second: f64,
    pub paused: bool,
}

/// A full in-game day is 2400 ticks (1200 day, 1200 night).
const TICKS_PER_DAY: u64 = 2400;
const DAY_HALF: u64 = 1200;

impl GameTime {
    /// Create a new game clock starting at tick 0.
    pub fn new(ticks_per_second: f64) -> Self {
        assert!(ticks_per_second > 0.0, "ticks_per_second must be positive");
        Self {
            tick: 0,
            ticks_per_second,
            paused: false,
        }
    }

    /// Advance by one tick (no-op when paused).
    pub fn advance(&mut self) {
        if !self.paused {
            self.tick += 1;
        }
    }

    /// Advance by `ticks` ticks (no-op when paused).
    pub fn advance_by(&mut self, ticks: u64) {
        if !self.paused {
            self.tick += ticks;
        }
    }

    /// Elapsed real-world seconds.
    pub fn real_seconds(&self) -> f64 {
        self.tick as f64 / self.ticks_per_second
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// True during the night half of a day cycle.
    pub fn is_night(&self) -> bool {
        self.tick % TICKS_PER_DAY > DAY_HALF
    }

    /// 0.0 = midnight, 0.5 = noon, 1.0 = midnight again.
    pub fn time_of_day(&self) -> f64 {
        (self.tick % TICKS_PER_DAY) as f64 / TICKS_PER_DAY as f64
    }

    /// Which day number we're on (0-indexed).
    pub fn day_number(&self) -> u64 {
        self.tick / TICKS_PER_DAY
    }
}

impl Default for GameTime {
    fn default() -> Self {
        Self::new(10.0)
    }
}

// ---------------------------------------------------------------------------
// TimeOfDay
// ---------------------------------------------------------------------------

/// Named phases of a day cycle, derived from the current tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeOfDay {
    Dawn,
    Morning,
    Midday,
    Afternoon,
    Dusk,
    Evening,
    Night,
    Midnight,
}

impl TimeOfDay {
    pub fn from_game_time(time: &GameTime) -> Self {
        let tod = time.time_of_day();
        // 0.0 = midnight, 0.5 = noon
        match tod {
            t if t < 0.04 => TimeOfDay::Midnight,
            t if t < 0.17 => TimeOfDay::Dawn,
            t if t < 0.33 => TimeOfDay::Morning,
            t if t < 0.46 => TimeOfDay::Midday,
            t if t < 0.54 => TimeOfDay::Afternoon,
            t if t < 0.67 => TimeOfDay::Dusk,
            t if t < 0.83 => TimeOfDay::Evening,
            t if t < 0.96 => TimeOfDay::Night,
            _ => TimeOfDay::Midnight,
        }
    }
}

// ---------------------------------------------------------------------------
// TimeDilation
// ---------------------------------------------------------------------------

/// Controls how fast game time flows relative to the base clock.
/// 1.0 = normal, 2.0 = double speed, 0.5 = half speed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeDilation {
    pub factor: f64,
}

impl TimeDilation {
    pub fn new(factor: f64) -> Self {
        assert!(factor > 0.0, "dilation factor must be positive");
        Self { factor }
    }

    /// Derive dilation from a room's vibe level.
    /// `vibe` is expected in roughly -1.0..1.0 range.
    /// High vibe → faster (more happening), low → slower (contemplative).
    pub fn from_vibe(vibe: f64) -> Self {
        let factor = 1.0 + vibe * 0.5;
        Self {
            factor: factor.max(0.1),
        }
    }

    pub fn is_accelerated(&self) -> bool {
        self.factor > 1.0
    }

    pub fn is_slowed(&self) -> bool {
        self.factor < 1.0
    }
}

impl Default for TimeDilation {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

// ---------------------------------------------------------------------------
// ScheduledEvent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduledEvent {
    QuestStart(String),
    RoomDissolve(String),
    WeatherChange(String),
    AgentMilestone(String, f64),
    SeasonChange(String),
}

// ---------------------------------------------------------------------------
// Schedule
// ---------------------------------------------------------------------------

/// A tick-indexed calendar of upcoming events.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schedule {
    events: BTreeMap<u64, ScheduledEvent>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_event(&mut self, tick: u64, event: ScheduledEvent) {
        self.events.insert(tick, event);
    }

    /// Pop and return all events whose tick ≤ `current_tick`.
    pub fn due_events(&mut self, current_tick: u64) -> Vec<ScheduledEvent> {
        let due_keys: Vec<u64> = self
            .events
            .keys()
            .copied()
            .filter(|&k| k <= current_tick)
            .collect();
        due_keys
            .into_iter()
            .filter_map(|k| self.events.remove(&k))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- GameTime basics -------------------------------------------------------

    #[test]
    fn game_time_new() {
        let gt = GameTime::new(20.0);
        assert_eq!(gt.tick, 0);
        assert!(!gt.paused);
        assert_eq!(gt.ticks_per_second, 20.0);
    }

    #[test]
    fn game_time_default() {
        let gt = GameTime::default();
        assert_eq!(gt.ticks_per_second, 10.0);
    }

    #[test]
    fn advance_tick() {
        let mut gt = GameTime::new(10.0);
        gt.advance();
        gt.advance();
        assert_eq!(gt.tick, 2);
    }

    #[test]
    fn advance_by_tick() {
        let mut gt = GameTime::new(10.0);
        gt.advance_by(50);
        assert_eq!(gt.tick, 50);
    }

    #[test]
    fn advance_ignores_paused() {
        let mut gt = GameTime::new(10.0);
        gt.pause();
        gt.advance();
        gt.advance_by(10);
        assert_eq!(gt.tick, 0);
        gt.resume();
        gt.advance();
        assert_eq!(gt.tick, 1);
    }

    #[test]
    fn real_seconds() {
        let mut gt = GameTime::new(10.0);
        gt.advance_by(100);
        assert!((gt.real_seconds() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn is_night_daytime() {
        let mut gt = GameTime::new(10.0);
        gt.advance_by(600); // within first 1200 ticks → day
        assert!(!gt.is_night());
    }

    #[test]
    fn is_night_nighttime() {
        let mut gt = GameTime::new(10.0);
        gt.advance_by(1500); // tick 1500 % 2400 = 1500 > 1200
        assert!(gt.is_night());
    }

    #[test]
    fn is_night_wraps_day_boundary() {
        let mut gt = GameTime::new(10.0);
        gt.advance_by(2400); // exactly 2400 → tick 0 mod 2400 → not night
        assert!(!gt.is_night());
    }

    #[test]
    fn time_of_day_midnight() {
        let gt = GameTime::new(10.0);
        assert!((gt.time_of_day()).abs() < f64::EPSILON);
    }

    #[test]
    fn time_of_day_noon() {
        let mut gt = GameTime::new(10.0);
        gt.advance_by(1200);
        assert!((gt.time_of_day() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn day_number() {
        let mut gt = GameTime::new(10.0);
        assert_eq!(gt.day_number(), 0);
        gt.advance_by(2400);
        assert_eq!(gt.day_number(), 1);
        gt.advance_by(2400 * 3);
        assert_eq!(gt.day_number(), 4);
    }

    // -- TimeOfDay -------------------------------------------------------------

    #[test]
    fn time_of_day_enum_midnight() {
        let gt = GameTime::new(10.0); // tick 0 → tod 0.0
        assert_eq!(TimeOfDay::from_game_time(&gt), TimeOfDay::Midnight);
    }

    #[test]
    fn time_of_day_enum_morning() {
        let mut gt = GameTime::new(10.0);
        gt.tick = 500; // 500/2400 ≈ 0.208 → Morning
        assert_eq!(TimeOfDay::from_game_time(&gt), TimeOfDay::Morning);
    }

    #[test]
    fn time_of_day_enum_afternoon() {
        let mut gt = GameTime::new(10.0);
        gt.tick = 1200; // 0.5 → Afternoon
        assert_eq!(TimeOfDay::from_game_time(&gt), TimeOfDay::Afternoon);
    }

    #[test]
    fn time_of_day_enum_night() {
        let mut gt = GameTime::new(10.0);
        gt.tick = 2000; // 2000/2400 ≈ 0.833 → Night
        assert_eq!(TimeOfDay::from_game_time(&gt), TimeOfDay::Night);
    }

    // -- TimeDilation ----------------------------------------------------------

    #[test]
    fn dilation_default() {
        let d = TimeDilation::default();
        assert!((d.factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dilation_from_vibe_neutral() {
        let d = TimeDilation::from_vibe(0.0);
        assert!((d.factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dilation_from_vibe_high() {
        let d = TimeDilation::from_vibe(1.0);
        assert!((d.factor - 1.5).abs() < f64::EPSILON);
        assert!(d.is_accelerated());
    }

    #[test]
    fn dilation_from_vibe_low() {
        let d = TimeDilation::from_vibe(-1.0);
        assert!((d.factor - 0.5).abs() < f64::EPSILON);
        assert!(d.is_slowed());
    }

    #[test]
    fn dilation_from_vibe_clamped() {
        let d = TimeDilation::from_vibe(-10.0);
        assert!(d.factor >= 0.1);
    }

    // -- Schedule --------------------------------------------------------------

    #[test]
    fn schedule_add_and_due() {
        let mut sched = Schedule::new();
        sched.add_event(10, ScheduledEvent::QuestStart("q1".into()));
        sched.add_event(20, ScheduledEvent::RoomDissolve("r1".into()));
        let due = sched.due_events(15);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0], ScheduledEvent::QuestStart("q1".into()));
        assert_eq!(sched.len(), 1);
    }

    #[test]
    fn schedule_due_multiple() {
        let mut sched = Schedule::new();
        sched.add_event(5, ScheduledEvent::WeatherChange("rain".into()));
        sched.add_event(10, ScheduledEvent::SeasonChange("winter".into()));
        sched.add_event(15, ScheduledEvent::AgentMilestone("a1".into(), 0.8));
        let due = sched.due_events(10);
        assert_eq!(due.len(), 2);
        assert!(sched.is_empty() == false);
        let rest = sched.due_events(100);
        assert_eq!(rest.len(), 1);
        assert!(sched.is_empty());
    }

    #[test]
    fn schedule_nothing_due() {
        let mut sched = Schedule::new();
        sched.add_event(100, ScheduledEvent::QuestStart("late".into()));
        assert!(sched.due_events(50).is_empty());
    }

    // -- Serde round-trip ------------------------------------------------------

    #[test]
    fn serde_game_time() {
        let gt = GameTime::new(42.0);
        let json = serde_json::to_string(&gt).unwrap();
        let gt2: GameTime = serde_json::from_str(&json).unwrap();
        assert_eq!(gt.tick, gt2.tick);
        assert!((gt.ticks_per_second - gt2.ticks_per_second).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_scheduled_event() {
        let ev = ScheduledEvent::AgentMilestone("agent-7".into(), 3.14);
        let json = serde_json::to_string(&ev).unwrap();
        let ev2: ScheduledEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, ev2);
    }
}
