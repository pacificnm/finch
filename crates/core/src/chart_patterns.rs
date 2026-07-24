//! Deterministic chart-pattern detection.
//!
//! Reduces a candle series to its zigzag swing pivots, then matches classic
//! chart-pattern geometry (double top/bottom, head & shoulders, triangles)
//! against the pivot sequence. Every match is rule-based — a concrete set of
//! tolerance checks — rather than a black-box classifier, because the point
//! of this feature is to let the AI *explain* why a pattern qualifies (this
//! is a chart-reading teaching tool, not just a labeling tool).

use serde::Serialize;

/// One OHLC bar. Deliberately decoupled from Schwab's raw JSON shape so the
/// pattern math doesn't need to know about `serde_json::Value` field lookups.
#[derive(Debug, Clone)]
pub(crate) struct Candle {
    pub date: String,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PivotKind {
    High,
    Low,
}

/// A confirmed (or, for the very last one, still-forming) swing point.
#[derive(Debug, Clone)]
pub(crate) struct Pivot {
    pub index: usize,
    pub date: String,
    pub price: f64,
    pub kind: PivotKind,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PatternPoint {
    pub date: String,
    pub price: f64,
    pub role: &'static str,
    /// "high" or "low" — the frontend uses this (not `role`, whose meaning
    /// differs between e.g. head & shoulders and its inverse) to decide
    /// whether to draw the marker above or below the bar.
    pub kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LinePoint {
    pub date: String,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PatternLine {
    pub role: &'static str,
    pub from: LinePoint,
    pub to: LinePoint,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PatternMatch {
    pub kind: &'static str,
    /// "confirmed" once price has closed through the neckline/trendline,
    /// "forming" while the shape is present but not yet broken.
    pub status: &'static str,
    pub label: String,
    /// A deterministic, fact-based description built from the actual
    /// measured points — this is what the model paraphrases into chat
    /// prose, never something it should invent or recompute.
    pub note: String,
    pub points: Vec<PatternPoint>,
    pub lines: Vec<PatternLine>,
}

const ZIGZAG_ATR_MULTIPLIER: f64 = 2.0;
/// Fallback swing threshold, as a fraction of the latest close, used when
/// there isn't enough history for a stable ATR.
const ZIGZAG_FALLBACK_PCT: f64 = 0.03;
/// Double top/bottom: how close the two outer peaks/troughs must be to each
/// other, as a fraction of their average price.
const DOUBLE_PEAK_TOLERANCE_PCT: f64 = 0.03;
/// Double top/bottom: minimum retracement from the peaks to the middle
/// pivot, as a fraction of the peak price — rejects noise-level wiggles.
const DOUBLE_MIN_RETRACEMENT_PCT: f64 = 0.03;
/// Head & shoulders: how close the two shoulders must be to each other, as
/// a fraction of their average price.
const HS_SHOULDER_SYMMETRY_PCT: f64 = 0.05;
/// Triangles: a trendline's per-bar slope, as a fraction of price, at or
/// below which it counts as "flat" rather than rising/falling.
const TRIANGLE_FLAT_SLOPE_PCT: f64 = 0.0015;

/// Detects chart patterns in `candles`. `atr` should be the current
/// Average True Range (in price units, not percent) if available — it sets
/// the zigzag's noise threshold; pass `None` to fall back to a flat
/// percentage of price. `requested_kinds`, if given, filters the result to
/// only those pattern kinds (see `PatternMatch::kind` for the strings).
pub(crate) fn detect_patterns(
    candles: &[Candle],
    atr: Option<f64>,
    requested_kinds: Option<&[&str]>,
) -> Vec<PatternMatch> {
    if candles.len() < 6 {
        return Vec::new();
    }

    let threshold = zigzag_threshold(candles, atr);
    let pivots = find_zigzag_pivots(candles, threshold);
    let last_index = candles.len() - 1;
    let last_close = candles[last_index].close;

    let mut matches = Vec::new();
    matches.extend(detect_double_top_bottom(&pivots, last_close));
    matches.extend(detect_head_and_shoulders(&pivots, last_index, last_close));
    matches.extend(detect_triangles(&pivots, last_index, last_close));

    if let Some(kinds) = requested_kinds {
        matches.retain(|m| kinds.contains(&m.kind));
    }

    matches
}

fn zigzag_threshold(candles: &[Candle], atr: Option<f64>) -> f64 {
    if let Some(atr) = atr {
        if atr > 0.0 {
            return atr * ZIGZAG_ATR_MULTIPLIER;
        }
    }
    let last_close = candles.last().map(|c| c.close).unwrap_or(0.0);
    last_close * ZIGZAG_FALLBACK_PCT
}

/// ATR/percentage-scaled zigzag: walks the candles once, tracking a running
/// candidate extreme in the current search direction. A new extreme in that
/// direction extends the candidate; a retracement of `threshold` or more
/// confirms it as a pivot and flips direction. The final running candidate
/// is pushed unconfirmed, so patterns anchored at the most recent swing
/// (e.g. a still-forming right shoulder) are visible.
pub(crate) fn find_zigzag_pivots(candles: &[Candle], threshold: f64) -> Vec<Pivot> {
    if candles.len() < 2 || threshold <= 0.0 {
        return Vec::new();
    }

    // Phase 1: establish the first confirmed swing by tracking both the
    // running high and running low from the start until price has moved
    // `threshold` away from one of them.
    let mut high_idx = 0usize;
    let mut low_idx = 0usize;
    let mut first_kind: Option<PivotKind> = None;
    let mut start = candles.len();

    for i in 1..candles.len() {
        if candles[i].high > candles[high_idx].high {
            high_idx = i;
        }
        if candles[i].low < candles[low_idx].low {
            low_idx = i;
        }

        let drop_from_high = candles[high_idx].high - candles[i].low;
        if drop_from_high >= threshold && high_idx < i {
            first_kind = Some(PivotKind::High);
            start = i;
            break;
        }

        let rally_from_low = candles[i].high - candles[low_idx].low;
        if rally_from_low >= threshold && low_idx < i {
            first_kind = Some(PivotKind::Low);
            start = i;
            break;
        }
    }

    let Some(first_kind) = first_kind else {
        return Vec::new();
    };

    let (mut pivot_idx, mut pivot_price, mut pivot_kind) = match first_kind {
        PivotKind::High => (high_idx, candles[high_idx].high, PivotKind::High),
        PivotKind::Low => (low_idx, candles[low_idx].low, PivotKind::Low),
    };

    // Phase 2: extend/confirm/flip forward from the first swing.
    let mut pivots = Vec::new();
    for i in start..candles.len() {
        let c = &candles[i];
        match pivot_kind {
            PivotKind::High => {
                if c.high > pivot_price {
                    pivot_idx = i;
                    pivot_price = c.high;
                } else if pivot_price - c.low >= threshold {
                    pivots.push(make_pivot(candles, pivot_idx, pivot_price, PivotKind::High));
                    pivot_kind = PivotKind::Low;
                    pivot_idx = i;
                    pivot_price = c.low;
                }
            }
            PivotKind::Low => {
                if c.low < pivot_price {
                    pivot_idx = i;
                    pivot_price = c.low;
                } else if c.high - pivot_price >= threshold {
                    pivots.push(make_pivot(candles, pivot_idx, pivot_price, PivotKind::Low));
                    pivot_kind = PivotKind::High;
                    pivot_idx = i;
                    pivot_price = c.high;
                }
            }
        }
    }

    pivots.push(make_pivot(candles, pivot_idx, pivot_price, pivot_kind));
    pivots
}

fn make_pivot(candles: &[Candle], index: usize, price: f64, kind: PivotKind) -> Pivot {
    Pivot {
        index,
        date: candles[index].date.clone(),
        price,
        kind,
    }
}

fn pivot_role(kind: PivotKind) -> &'static str {
    match kind {
        PivotKind::High => "swing_high",
        PivotKind::Low => "swing_low",
    }
}

fn point_kind(kind: PivotKind) -> &'static str {
    match kind {
        PivotKind::High => "high",
        PivotKind::Low => "low",
    }
}

/// Value of the line through `from` and `to` at bar index `at_index`
/// (extrapolated if `at_index` is outside `[from.index, to.index]`).
fn extrapolate_line(from: &Pivot, to: &Pivot, at_index: usize) -> f64 {
    let x0 = from.index as f64;
    let x1 = to.index as f64;
    if (x1 - x0).abs() < f64::EPSILON {
        return from.price;
    }
    let x = at_index as f64;
    from.price + (to.price - from.price) * (x - x0) / (x1 - x0)
}

fn slope_per_bar(from: &Pivot, to: &Pivot) -> f64 {
    let dx = to.index as f64 - from.index as f64;
    if dx.abs() < f64::EPSILON {
        return 0.0;
    }
    (to.price - from.price) / dx
}

fn detect_double_top_bottom(pivots: &[Pivot], last_close: f64) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    for w in pivots.windows(3) {
        let [a, b, c] = w else { continue };
        match (a.kind, b.kind, c.kind) {
            (PivotKind::High, PivotKind::Low, PivotKind::High) => {
                if let Some(m) = try_double_top(a, b, c, last_close) {
                    matches.push(m);
                }
            }
            (PivotKind::Low, PivotKind::High, PivotKind::Low) => {
                if let Some(m) = try_double_bottom(a, b, c, last_close) {
                    matches.push(m);
                }
            }
            _ => {}
        }
    }
    matches
}

fn try_double_top(left: &Pivot, trough: &Pivot, right: &Pivot, last_close: f64) -> Option<PatternMatch> {
    let peak_avg = (left.price + right.price) / 2.0;
    if peak_avg <= 0.0 {
        return None;
    }
    let peak_diff_pct = (left.price - right.price).abs() / peak_avg;
    if peak_diff_pct > DOUBLE_PEAK_TOLERANCE_PCT {
        return None;
    }
    let retracement_pct = (peak_avg - trough.price) / peak_avg;
    if retracement_pct < DOUBLE_MIN_RETRACEMENT_PCT {
        return None;
    }

    let neckline = trough.price;
    let status = if last_close < neckline { "confirmed" } else { "forming" };
    let note = format!(
        "Two peaks near ${:.2} on {} and ${:.2} on {} (within {:.1}% of each other), separated by a pullback to ${:.2} on {} — a {:.1}% retracement from the peaks. {}",
        left.price, left.date, right.price, right.date, peak_diff_pct * 100.0,
        trough.price, trough.date, retracement_pct * 100.0,
        if status == "confirmed" {
            "Price has since closed below the neckline, confirming the pattern."
        } else {
            "Price hasn't closed below the neckline yet, so this pattern is still forming."
        }
    );

    Some(PatternMatch {
        kind: "double_top",
        status,
        label: "Double Top".to_string(),
        note,
        points: vec![
            PatternPoint { date: left.date.clone(), price: left.price, role: "first_peak", kind: "high" },
            PatternPoint { date: trough.date.clone(), price: trough.price, role: "trough", kind: "low" },
            PatternPoint { date: right.date.clone(), price: right.price, role: "second_peak", kind: "high" },
        ],
        lines: vec![PatternLine {
            role: "neckline",
            from: LinePoint { date: left.date.clone(), price: neckline },
            to: LinePoint { date: right.date.clone(), price: neckline },
        }],
    })
}

fn try_double_bottom(left: &Pivot, peak: &Pivot, right: &Pivot, last_close: f64) -> Option<PatternMatch> {
    let trough_avg = (left.price + right.price) / 2.0;
    if trough_avg <= 0.0 {
        return None;
    }
    let trough_diff_pct = (left.price - right.price).abs() / trough_avg;
    if trough_diff_pct > DOUBLE_PEAK_TOLERANCE_PCT {
        return None;
    }
    let retracement_pct = (peak.price - trough_avg) / trough_avg;
    if retracement_pct < DOUBLE_MIN_RETRACEMENT_PCT {
        return None;
    }

    let neckline = peak.price;
    let status = if last_close > neckline { "confirmed" } else { "forming" };
    let note = format!(
        "Two troughs near ${:.2} on {} and ${:.2} on {} (within {:.1}% of each other), separated by a rally to ${:.2} on {} — a {:.1}% bounce from the troughs. {}",
        left.price, left.date, right.price, right.date, trough_diff_pct * 100.0,
        peak.price, peak.date, retracement_pct * 100.0,
        if status == "confirmed" {
            "Price has since closed above the neckline, confirming the pattern."
        } else {
            "Price hasn't closed above the neckline yet, so this pattern is still forming."
        }
    );

    Some(PatternMatch {
        kind: "double_bottom",
        status,
        label: "Double Bottom".to_string(),
        note,
        points: vec![
            PatternPoint { date: left.date.clone(), price: left.price, role: "first_trough", kind: "low" },
            PatternPoint { date: peak.date.clone(), price: peak.price, role: "peak", kind: "high" },
            PatternPoint { date: right.date.clone(), price: right.price, role: "second_trough", kind: "low" },
        ],
        lines: vec![PatternLine {
            role: "neckline",
            from: LinePoint { date: left.date.clone(), price: neckline },
            to: LinePoint { date: right.date.clone(), price: neckline },
        }],
    })
}

fn detect_head_and_shoulders(pivots: &[Pivot], last_index: usize, last_close: f64) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    for w in pivots.windows(5) {
        let [a, b, c, d, e] = w else { continue };
        match (a.kind, b.kind, c.kind, d.kind, e.kind) {
            (PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High) => {
                if let Some(m) = try_head_and_shoulders(a, b, c, d, e, last_index, last_close) {
                    matches.push(m);
                }
            }
            (PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low) => {
                if let Some(m) = try_inverse_head_and_shoulders(a, b, c, d, e, last_index, last_close) {
                    matches.push(m);
                }
            }
            _ => {}
        }
    }
    matches
}

#[allow(clippy::too_many_arguments)]
fn try_head_and_shoulders(
    left_shoulder: &Pivot,
    trough1: &Pivot,
    head: &Pivot,
    trough2: &Pivot,
    right_shoulder: &Pivot,
    last_index: usize,
    last_close: f64,
) -> Option<PatternMatch> {
    if !(head.price > left_shoulder.price && head.price > right_shoulder.price) {
        return None;
    }
    let shoulder_avg = (left_shoulder.price + right_shoulder.price) / 2.0;
    if shoulder_avg <= 0.0 {
        return None;
    }
    let shoulder_diff_pct = (left_shoulder.price - right_shoulder.price).abs() / shoulder_avg;
    if shoulder_diff_pct > HS_SHOULDER_SYMMETRY_PCT {
        return None;
    }

    let neckline_now = extrapolate_line(trough1, trough2, last_index);
    let status = if last_close < neckline_now { "confirmed" } else { "forming" };
    let note = format!(
        "Left shoulder ${:.2} on {}, head ${:.2} on {} (the highest point), right shoulder ${:.2} on {} (within {:.1}% of the left shoulder), neckline through the two troughs at ${:.2} on {} and ${:.2} on {}. {}",
        left_shoulder.price, left_shoulder.date, head.price, head.date,
        right_shoulder.price, right_shoulder.date, shoulder_diff_pct * 100.0,
        trough1.price, trough1.date, trough2.price, trough2.date,
        if status == "confirmed" {
            "Price has since closed below the neckline, confirming the pattern."
        } else {
            "Price hasn't closed below the neckline yet, so this pattern is still forming."
        }
    );

    Some(PatternMatch {
        kind: "head_and_shoulders",
        status,
        label: "Head & Shoulders".to_string(),
        note,
        points: vec![
            PatternPoint { date: left_shoulder.date.clone(), price: left_shoulder.price, role: "left_shoulder", kind: "high" },
            PatternPoint { date: trough1.date.clone(), price: trough1.price, role: "left_trough", kind: "low" },
            PatternPoint { date: head.date.clone(), price: head.price, role: "head", kind: "high" },
            PatternPoint { date: trough2.date.clone(), price: trough2.price, role: "right_trough", kind: "low" },
            PatternPoint { date: right_shoulder.date.clone(), price: right_shoulder.price, role: "right_shoulder", kind: "high" },
        ],
        lines: vec![PatternLine {
            role: "neckline",
            from: LinePoint { date: trough1.date.clone(), price: trough1.price },
            to: LinePoint { date: trough2.date.clone(), price: trough2.price },
        }],
    })
}

#[allow(clippy::too_many_arguments)]
fn try_inverse_head_and_shoulders(
    left_shoulder: &Pivot,
    peak1: &Pivot,
    head: &Pivot,
    peak2: &Pivot,
    right_shoulder: &Pivot,
    last_index: usize,
    last_close: f64,
) -> Option<PatternMatch> {
    if !(head.price < left_shoulder.price && head.price < right_shoulder.price) {
        return None;
    }
    let shoulder_avg = (left_shoulder.price + right_shoulder.price) / 2.0;
    if shoulder_avg <= 0.0 {
        return None;
    }
    let shoulder_diff_pct = (left_shoulder.price - right_shoulder.price).abs() / shoulder_avg;
    if shoulder_diff_pct > HS_SHOULDER_SYMMETRY_PCT {
        return None;
    }

    let neckline_now = extrapolate_line(peak1, peak2, last_index);
    let status = if last_close > neckline_now { "confirmed" } else { "forming" };
    let note = format!(
        "Left shoulder ${:.2} on {}, head ${:.2} on {} (the lowest point), right shoulder ${:.2} on {} (within {:.1}% of the left shoulder), neckline through the two peaks at ${:.2} on {} and ${:.2} on {}. {}",
        left_shoulder.price, left_shoulder.date, head.price, head.date,
        right_shoulder.price, right_shoulder.date, shoulder_diff_pct * 100.0,
        peak1.price, peak1.date, peak2.price, peak2.date,
        if status == "confirmed" {
            "Price has since closed above the neckline, confirming the pattern."
        } else {
            "Price hasn't closed above the neckline yet, so this pattern is still forming."
        }
    );

    Some(PatternMatch {
        kind: "inverse_head_and_shoulders",
        status,
        label: "Inverse Head & Shoulders".to_string(),
        note,
        points: vec![
            PatternPoint { date: left_shoulder.date.clone(), price: left_shoulder.price, role: "left_shoulder", kind: "low" },
            PatternPoint { date: peak1.date.clone(), price: peak1.price, role: "left_peak", kind: "high" },
            PatternPoint { date: head.date.clone(), price: head.price, role: "head", kind: "low" },
            PatternPoint { date: peak2.date.clone(), price: peak2.price, role: "right_peak", kind: "high" },
            PatternPoint { date: right_shoulder.date.clone(), price: right_shoulder.price, role: "right_shoulder", kind: "low" },
        ],
        lines: vec![PatternLine {
            role: "neckline",
            from: LinePoint { date: peak1.date.clone(), price: peak1.price },
            to: LinePoint { date: peak2.date.clone(), price: peak2.price },
        }],
    })
}

fn detect_triangles(pivots: &[Pivot], last_index: usize, last_close: f64) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    for w in pivots.windows(4) {
        let [a, b, c, d] = w else { continue };
        if let Some(m) = try_triangle(a, b, c, d, last_index, last_close) {
            matches.push(m);
        }
    }
    matches
}

fn try_triangle(a: &Pivot, b: &Pivot, c: &Pivot, d: &Pivot, last_index: usize, last_close: f64) -> Option<PatternMatch> {
    let window = [a, b, c, d];
    let highs: Vec<&Pivot> = window.iter().copied().filter(|p| p.kind == PivotKind::High).collect();
    let lows: Vec<&Pivot> = window.iter().copied().filter(|p| p.kind == PivotKind::Low).collect();
    if highs.len() != 2 || lows.len() != 2 {
        return None;
    }

    let avg_price = (a.price + b.price + c.price + d.price) / 4.0;
    if avg_price <= 0.0 {
        return None;
    }

    let high_slope_pct = slope_per_bar(highs[0], highs[1]) / avg_price;
    let low_slope_pct = slope_per_bar(lows[0], lows[1]) / avg_price;

    let high_flat = high_slope_pct.abs() <= TRIANGLE_FLAT_SLOPE_PCT;
    let low_flat = low_slope_pct.abs() <= TRIANGLE_FLAT_SLOPE_PCT;
    let high_falling = high_slope_pct < -TRIANGLE_FLAT_SLOPE_PCT;
    let low_rising = low_slope_pct > TRIANGLE_FLAT_SLOPE_PCT;

    let (kind, label): (&'static str, &'static str) = if high_flat && low_rising {
        ("ascending_triangle", "Ascending Triangle")
    } else if low_flat && high_falling {
        ("descending_triangle", "Descending Triangle")
    } else if high_falling && low_rising {
        ("symmetrical_triangle", "Symmetrical Triangle")
    } else {
        return None;
    };

    let upper_now = extrapolate_line(highs[0], highs[1], last_index);
    let lower_now = extrapolate_line(lows[0], lows[1], last_index);
    let status = if last_close > upper_now || last_close < lower_now { "confirmed" } else { "forming" };

    let describe_slope = |flat: bool, falling: bool| {
        if flat {
            "roughly flat"
        } else if falling {
            "falling"
        } else {
            "rising"
        }
    };

    let note = format!(
        "Upper trendline through ${:.2} on {} and ${:.2} on {} is {}. Lower trendline through ${:.2} on {} and ${:.2} on {} is {}. {}",
        highs[0].price, highs[0].date, highs[1].price, highs[1].date,
        describe_slope(high_flat, high_falling),
        lows[0].price, lows[0].date, lows[1].price, lows[1].date,
        describe_slope(low_flat, low_slope_pct < -TRIANGLE_FLAT_SLOPE_PCT),
        if status == "confirmed" {
            "Price has since closed outside the triangle, confirming a breakout."
        } else {
            "Price is still trading inside the triangle."
        }
    );

    Some(PatternMatch {
        kind,
        status,
        label: label.to_string(),
        note,
        points: vec![
            PatternPoint { date: a.date.clone(), price: a.price, role: pivot_role(a.kind), kind: point_kind(a.kind) },
            PatternPoint { date: b.date.clone(), price: b.price, role: pivot_role(b.kind), kind: point_kind(b.kind) },
            PatternPoint { date: c.date.clone(), price: c.price, role: pivot_role(c.kind), kind: point_kind(c.kind) },
            PatternPoint { date: d.date.clone(), price: d.price, role: pivot_role(d.kind), kind: point_kind(d.kind) },
        ],
        lines: vec![
            PatternLine {
                role: "upper_trendline",
                from: LinePoint { date: highs[0].date.clone(), price: highs[0].price },
                to: LinePoint { date: highs[1].date.clone(), price: highs[1].price },
            },
            PatternLine {
                role: "lower_trendline",
                from: LinePoint { date: lows[0].date.clone(), price: lows[0].price },
                to: LinePoint { date: lows[1].date.clone(), price: lows[1].price },
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(date: &str, high: f64, low: f64, close: f64) -> Candle {
        Candle { date: date.to_string(), high, low, close }
    }

    fn pivot(index: usize, date: &str, price: f64, kind: PivotKind) -> Pivot {
        Pivot { index, date: date.to_string(), price, kind }
    }

    #[test]
    fn find_zigzag_pivots_reduces_a_monotonic_uptrend_to_its_two_endpoints() {
        let candles = vec![
            candle("d0", 10.0, 9.0, 9.5),
            candle("d1", 11.0, 10.0, 10.5),
            candle("d2", 12.0, 11.0, 11.5),
            candle("d3", 13.0, 12.0, 12.5),
            candle("d4", 14.0, 13.0, 13.5),
            candle("d5", 15.0, 14.0, 14.5),
        ];
        let pivots = find_zigzag_pivots(&candles, 2.0);
        assert_eq!(pivots.len(), 2, "{pivots:?}");
        assert_eq!(pivots[0].kind, PivotKind::Low);
        assert!((pivots[0].price - 9.0).abs() < 1e-9);
        assert_eq!(pivots[1].kind, PivotKind::High);
        assert!((pivots[1].price - 15.0).abs() < 1e-9);
    }

    #[test]
    fn find_zigzag_pivots_traces_a_v_shape_as_three_pivots() {
        let candles = vec![
            candle("d0", 20.0, 19.0, 19.5),
            candle("d1", 18.0, 17.0, 17.5),
            candle("d2", 16.0, 15.0, 15.5),
            candle("d3", 14.0, 13.0, 13.5),
            candle("d4", 15.0, 14.0, 14.5),
            candle("d5", 17.0, 16.0, 16.5),
            candle("d6", 19.0, 18.0, 18.5),
        ];
        let pivots = find_zigzag_pivots(&candles, 2.0);
        assert_eq!(pivots.len(), 3, "{pivots:?}");
        assert_eq!(pivots[0].kind, PivotKind::High);
        assert!((pivots[0].price - 20.0).abs() < 1e-9);
        assert_eq!(pivots[1].kind, PivotKind::Low);
        assert!((pivots[1].price - 13.0).abs() < 1e-9);
        assert_eq!(pivots[2].kind, PivotKind::High);
        assert!((pivots[2].price - 19.0).abs() < 1e-9);
    }

    #[test]
    fn find_zigzag_pivots_ignores_sub_threshold_noise() {
        let candles = vec![
            candle("d0", 101.0, 100.0, 100.5),
            candle("d1", 100.5, 99.7, 100.0),
            candle("d2", 101.2, 100.2, 100.8),
            candle("d3", 100.8, 99.9, 100.3),
            candle("d4", 101.5, 100.5, 101.0),
        ];
        assert!(find_zigzag_pivots(&candles, 5.0).is_empty());
    }

    #[test]
    fn find_zigzag_pivots_finds_a_double_top_shaped_pair_of_equal_peaks() {
        // Hand-traced: threshold=2.0 confirms Low@0(9), High@2(20),
        // Low@4(10), High@6(20), then pushes the final running Low@7(14)
        // unconfirmed. The High@2/Low@4/High@6 triple is a textbook double
        // top (equal peaks, clear trough between them).
        let candles = vec![
            candle("d0", 10.0, 9.0, 9.5),
            candle("d1", 15.0, 14.0, 14.5),
            candle("d2", 20.0, 19.0, 19.5),
            candle("d3", 16.0, 15.0, 15.5),
            candle("d4", 11.0, 10.0, 10.5),
            candle("d5", 16.0, 15.0, 15.5),
            candle("d6", 20.0, 19.0, 19.5),
            candle("d7", 15.0, 14.0, 14.5),
        ];
        let pivots = find_zigzag_pivots(&candles, 2.0);
        assert_eq!(pivots.len(), 5, "{pivots:?}");
        assert_eq!(pivots[1].kind, PivotKind::High);
        assert!((pivots[1].price - 20.0).abs() < 1e-9);
        assert_eq!(pivots[2].kind, PivotKind::Low);
        assert!((pivots[2].price - 10.0).abs() < 1e-9);
        assert_eq!(pivots[3].kind, PivotKind::High);
        assert!((pivots[3].price - 20.0).abs() < 1e-9);
    }

    #[test]
    fn try_double_top_matches_within_tolerance_and_reports_confirmed_breakdown() {
        let left = pivot(0, "d0", 100.0, PivotKind::High);
        let trough = pivot(5, "d5", 90.0, PivotKind::Low);
        let right = pivot(10, "d10", 101.0, PivotKind::High);

        let m = try_double_top(&left, &trough, &right, 85.0).expect("should match");
        assert_eq!(m.kind, "double_top");
        assert_eq!(m.status, "confirmed");
        assert_eq!(m.points.len(), 3);
        assert_eq!(m.lines.len(), 1);
    }

    #[test]
    fn try_double_top_rejects_peaks_outside_tolerance() {
        let left = pivot(0, "d0", 100.0, PivotKind::High);
        let trough = pivot(5, "d5", 90.0, PivotKind::Low);
        let right = pivot(10, "d10", 110.0, PivotKind::High);

        assert!(try_double_top(&left, &trough, &right, 85.0).is_none());
    }

    #[test]
    fn try_head_and_shoulders_matches_symmetric_shoulders() {
        let left_shoulder = pivot(0, "d0", 100.0, PivotKind::High);
        let trough1 = pivot(3, "d3", 90.0, PivotKind::Low);
        let head = pivot(6, "d6", 110.0, PivotKind::High);
        let trough2 = pivot(9, "d9", 91.0, PivotKind::Low);
        let right_shoulder = pivot(12, "d12", 101.0, PivotKind::High);

        let m = try_head_and_shoulders(&left_shoulder, &trough1, &head, &trough2, &right_shoulder, 15, 85.0)
            .expect("should match");
        assert_eq!(m.kind, "head_and_shoulders");
        assert_eq!(m.status, "confirmed");
        assert_eq!(m.points.len(), 5);
    }

    #[test]
    fn try_head_and_shoulders_rejects_asymmetric_shoulders() {
        let left_shoulder = pivot(0, "d0", 100.0, PivotKind::High);
        let trough1 = pivot(3, "d3", 90.0, PivotKind::Low);
        let head = pivot(6, "d6", 110.0, PivotKind::High);
        let trough2 = pivot(9, "d9", 91.0, PivotKind::Low);
        let right_shoulder = pivot(12, "d12", 130.0, PivotKind::High);

        assert!(
            try_head_and_shoulders(&left_shoulder, &trough1, &head, &trough2, &right_shoulder, 15, 85.0).is_none()
        );
    }

    #[test]
    fn try_triangle_matches_ascending_triangle() {
        let a = pivot(0, "d0", 90.0, PivotKind::Low);
        let b = pivot(1, "d1", 100.0, PivotKind::High);
        let c = pivot(2, "d2", 95.0, PivotKind::Low);
        let d = pivot(3, "d3", 100.1, PivotKind::High);

        let m = try_triangle(&a, &b, &c, &d, 3, 99.0).expect("should match");
        assert_eq!(m.kind, "ascending_triangle");
        assert_eq!(m.status, "forming");
        assert_eq!(m.lines.len(), 2);
    }

    #[test]
    fn try_triangle_rejects_a_parallel_rising_channel() {
        let a = pivot(0, "d0", 90.0, PivotKind::Low);
        let b = pivot(1, "d1", 100.0, PivotKind::High);
        let c = pivot(2, "d2", 100.0, PivotKind::Low);
        let d = pivot(3, "d3", 110.0, PivotKind::High);

        assert!(try_triangle(&a, &b, &c, &d, 3, 105.0).is_none());
    }
}
