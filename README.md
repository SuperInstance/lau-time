# lau-time

A game-time engine for the **Lau** ecosystem — tick-based clocks, day/night cycles, room-driven time dilation, and a tick-indexed event scheduler.

---

## What This Does

`lau-time` provides the temporal backbone for a game world:

- **GameTime** — A monotonic tick counter that maps ticks to real-world seconds. Pause, resume, advance one tick or many. Query whether it's day or night, which day you're on, and the fractional time-of-day.
- **TimeOfDay** — An enum (`Dawn`, `Morning`, `Midday`, …, `Midnight`) derived from the current tick, partitioning a 2400-tick day into eight named phases.
- **TimeDilation** — A speed multiplier (0.5× = slow, 1.0× = normal, 2.0× = fast) that can be derived from a room's *vibe* score, making contemplative rooms feel slower and exciting rooms feel faster.
- **Schedule** — A `BTreeMap`-backed calendar indexed by tick number. Add events, then drain all events due at or before the current tick. Used for quest starts, room dissolves, weather shifts, agent milestones, and season changes.

Everything is `Serialize`/`Deserialize` via **serde**, so game state snapshots round-trip through JSON (or any serde format) trivially.

---

## Key Idea

One tick = one beat of the game clock. A full day is **2400 ticks** (1200 day, 1200 night). By changing `ticks_per_second`, you control how fast a game day elapses in real time. At the default 10 ticks/sec, one in-game day passes in 4 real minutes.

Time dilation layers on top of that: a room with high vibe (busy, exciting) accelerates time by up to 1.5×, while a low-vibe room (quiet, contemplative) slows it to 0.5×. The scheduler then fires events at absolute tick positions, completely decoupled from real-world wall-clock time.

---

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
lau-time = "0.1"
```

Requires **Rust 2021 edition**.

### Dependencies

| crate  | why                 |
|--------|---------------------|
| `serde` | Derive `Serialize` / `Deserialize` on all public types |

`serde_json` is only needed in `[dev-dependencies]` for the round-trip tests.

---

## Quick Start

```rust
use lau_time::{GameTime, TimeOfDay, TimeDilation, Schedule, ScheduledEvent};

// --- Clock ---
let mut clock = GameTime::new(10.0); // 10 ticks per second
clock.advance_by(500);
println!("tick={}, real_s={:.1}", clock.tick, clock.real_seconds()); // tick=500, real_s=50.0
println!("day={}, tod={:.3}", clock.day_number(), clock.time_of_day()); // day=0, tod=0.208
println!("night? {}", clock.is_night()); // false

// --- Phase ---
let phase = TimeOfDay::from_game_time(&clock);
println!("phase={:?}", phase); // Morning

// --- Dilation from room vibe ---
let dil = TimeDilation::from_vibe(0.8); // exciting room
println!("dilation={:.2}x, accelerated={}", dil.factor, dil.is_accelerated()); // 1.40x

// --- Scheduler ---
let mut schedule = Schedule::new();
schedule.add_event(600, ScheduledEvent::QuestStart("find-the-key".into()));
schedule.add_event(1200, ScheduledEvent::WeatherChange("rain".into()));

clock.advance_by(200); // now at tick 700
for ev in schedule.due_events(clock.tick) {
    println!("event due: {:?}", ev); // QuestStart("find-the-key")
}
```

---

## API Reference

### `GameTime`

| method | description |
|--------|-------------|
| `new(ticks_per_second)` | Create a clock at tick 0. Panics if rate ≤ 0. |
| `default()` | 10 ticks/sec, unpaused. |
| `advance()` | +1 tick (no-op if paused). |
| `advance_by(n)` | +n ticks (no-op if paused). |
| `real_seconds()` → `f64` | Elapsed real seconds = `tick / ticks_per_second`. |
| `pause()` / `resume()` | Toggle the `paused` flag. |
| `is_night()` → `bool` | `true` when `tick % 2400 > 1200`. |
| `time_of_day()` → `f64` | `0.0` = midnight, `0.5` = noon, `1.0` = midnight again. |
| `day_number()` → `u64` | Zero-indexed day count (`tick / 2400`). |

All fields (`tick`, `ticks_per_second`, `paused`) are public and serialisable.

### `TimeOfDay`

```rust
pub enum TimeOfDay {
    Dawn, Morning, Midday, Afternoon,
    Dusk, Evening, Night, Midnight,
}
```

| method | description |
|--------|-------------|
| `from_game_time(&GameTime)` | Maps `time_of_day()` into one of eight phases. |

Boundary table (fractional day → phase):

| range | phase |
|-------|-------|
| `[0.00, 0.04)` | Midnight |
| `[0.04, 0.17)` | Dawn |
| `[0.17, 0.33)` | Morning |
| `[0.33, 0.46)` | Midday |
| `[0.46, 0.54)` | Afternoon |
| `[0.54, 0.67)` | Dusk |
| `[0.67, 0.83)` | Evening |
| `[0.83, 0.96)` | Night |
| `[0.96, 1.00)` | Midnight |

### `TimeDilation`

| method | description |
|--------|-------------|
| `new(factor)` | Arbitrary positive multiplier. |
| `default()` | Factor = 1.0. |
| `from_vibe(vibe)` | Maps a `[-1, 1]` vibe score → `[0.5, 1.5]` factor (clamped ≥ 0.1). |
| `is_accelerated()` | `factor > 1.0` |
| `is_slowed()` | `factor < 1.0` |

### `Schedule` & `ScheduledEvent`

```rust
pub enum ScheduledEvent {
    QuestStart(String),
    RoomDissolve(String),
    WeatherChange(String),
    AgentMilestone(String, f64),
    SeasonChange(String),
}
```

| method | description |
|--------|-------------|
| `Schedule::new()` | Empty calendar. |
| `add_event(tick, event)` | Insert at the given tick (replaces any existing event at that tick). |
| `due_events(current_tick)` → `Vec<ScheduledEvent>` | Remove and return all events at `tick ≤ current_tick`. |
| `len()` / `is_empty()` | Query remaining count. |

Backed by `BTreeMap<u64, ScheduledEvent>` for O(log n) insertion and efficient range-drain.

---

## How It Works

```
┌──────────────┐    advance()    ┌───────────────┐
│  GameTime     │ ──────────────→ │  tick += 1    │
│  tick: u64    │                 │  (if !paused) │
│  tps:  f64    │                 └───────────────┘
└──────┬───────┘
       │
       ├──→ time_of_day()  = (tick % 2400) / 2400
       ├──→ is_night()     = (tick % 2400) > 1200
       ├──→ day_number()   = tick / 2400
       │
       │   ┌────────────────┐    from_vibe(v)    ┌──────────────┐
       └──→│ TimeDilation    │ ←───────────────── │  Room vibe   │
           │  factor: f64    │                    │  [-1.0, 1.0] │
           └────────────────┘                    └──────────────┘
                    │
                    ▼  multiply base tick rate by factor

       ┌────────────────┐    due_events(tick)    ┌──────────────┐
       │  Schedule       │ ─────────────────────→ │  Vec<Event>  │
       │  BTreeMap       │   drain ≤ tick         │  (fired)     │
       └────────────────┘                        └──────────────┘
```

1. **Game loop** calls `advance()` (or `advance_by(dt * tps * dilation.factor)`) once per frame.
2. **GameTime** increments the monotonic tick counter; all downstream queries are pure functions of the current tick.
3. **TimeDilation** doesn't live inside `GameTime` — it's an external multiplier the game loop applies when deciding *how many ticks to advance this frame*. This keeps the clock itself deterministic and replayable.
4. **Schedule** is a priority queue disguised as a `BTreeMap`. Each tick, the game loop calls `due_events(current_tick)` to collect and dispatch any pending world events.

---

## The Math

### Tick ↔ Real Time

$$\text{real\_seconds} = \frac{\text{tick}}{\text{ticks\_per\_second}}$$

At default settings (10 tps), 2400 ticks = 240 seconds = **4 minutes per in-game day**.

### Day/Night Cycle

A day is **2400 ticks**, split evenly:

$$\text{is\_night} = \left(\text{tick} \bmod 2400\right) > 1200$$

The **time-of-day** fraction maps the cycle to `[0, 1)`:

$$\text{time\_of\_day} = \frac{\text{tick} \bmod 2400}{2400}$$

0.0 is midnight, 0.5 is noon, and the cycle repeats every 2400 ticks.

### Time Dilation from Vibe

The vibe score ∈ roughly `[-1, 1]` is linearly mapped:

$$\text{factor} = \max\!\left(0.1,\; 1.0 + 0.5 \cdot \text{vibe}\right)$$

| vibe | factor | effect |
|------|--------|--------|
| +1.0 | 1.5× | time runs 50 % faster |
|  0.0 | 1.0× | normal speed |
| −1.0 | 0.5× | time runs at half speed |
| −10  | 0.1× | clamped floor |

### Effective Tick Rate

$$\text{effective\_tps} = \text{ticks\_per\_second} \times \text{dilation\_factor}$$

At 10 base tps with `from_vibe(1.0)`: effective rate = 15 tps → one day passes in ~160 seconds instead of 240.

### Day Number

$$\text{day\_number} = \left\lfloor \frac{\text{tick}}{2400} \right\rfloor$$

Zero-indexed; day 0 starts at tick 0, day 1 at tick 2400, etc.

---

## Tests

**26 tests** covering:

- Clock creation, defaults, advancement, pause/resume
- Night detection, time-of-day fraction, day numbering
- All eight `TimeOfDay` phases
- Dilation from vibe (neutral, high, low, clamped extreme)
- Schedule add, due-single, due-multiple, nothing-due, emptiness
- Serde JSON round-trips for `GameTime` and `ScheduledEvent`

Run:

```bash
cargo test
```

---

## License

MIT
