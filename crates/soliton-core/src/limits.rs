/// How far past its slider a parameter may be taken when a value is typed in.
///
/// A slider covers the range a textbook problem would use. A typed value may go ten times above it and to a
/// tenth of its lower end, so long as the quantity keeps its sign. A pure number (an angle, a ratio, a count, a
/// share) keeps the slider's range, since past it the scene usually stops being the one described; so does a
/// place along a wire of fixed length. The playground applies the same rule, in `typedLimits`.
pub fn typed_limits(name: &str, unit: &str, min: f64, max: f64) -> (f64, f64) {
    const FIXED: [&str; 4] = ["", "°", "%", "×"];
    if FIXED.contains(&unit) || name == "jockey" {
        return (min, max);
    }
    let reach = 10.0;
    let high = if max > 0.0 { reach * max } else { max / reach };
    let low = if min > 0.0 { min / reach } else { reach * min };
    // Nothing is colder than absolute zero.
    if unit == "°C" {
        (low.max(-273.0), high)
    } else {
        (low, high)
    }
}

#[cfg(test)]
mod tests {
    use super::typed_limits;

    #[test]
    fn quantities_open_out_and_pure_numbers_do_not() {
        assert_eq!(typed_limits("speed", "m/s", 0.5, 4.0), (0.05, 40.0));
        assert_eq!(typed_limits("load", "kg", 0.0, 60.0), (0.0, 600.0));
        assert_eq!(typed_limits("charge", "μC", -3.0, 3.0), (-30.0, 30.0));
        assert_eq!(typed_limits("start", "°C", -40.0, 0.0), (-273.0, 0.0));
        assert_eq!(typed_limits("angle", "°", 5.0, 85.0), (5.0, 85.0));
        assert_eq!(typed_limits("ratio", "", 1.5, 5.0), (1.5, 5.0));
        assert_eq!(typed_limits("jockey", "cm", 5.0, 95.0), (5.0, 95.0));
    }
}
