#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Config {
    pub samples: usize,
    pub warmup: usize,
    pub max_p99_ns: Option<u128>,
}
impl Config {
    pub fn accepts_total(self, distribution: Distribution) -> bool {
        self.max_p99_ns
            .is_none_or(|budget| distribution.p99 <= budget)
    }
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut config = Self {
            samples: 512,
            warmup: 32,
            max_p99_ns: None,
        };
        let mut args = args.into_iter();
        let mut seen = [false; 3];
        while let Some(key) = args.next() {
            let index = match key.as_str() {
                "--samples" => 0,
                "--warmup" => 1,
                "--max-p99-ns" => 2,
                _ => return Err("unknown argument".into()),
            };
            if seen[index] {
                return Err("duplicate argument".into());
            }
            seen[index] = true;
            let value = args.next().ok_or("missing numeric argument")?;
            if value.is_empty() || !value.bytes().all(|c| c.is_ascii_digit()) {
                return Err("expected unsigned decimal integer".into());
            }
            match index {
                0 => config.samples = value.parse().map_err(|_| "invalid sample count")?,
                1 => config.warmup = value.parse().map_err(|_| "invalid warmup count")?,
                _ => config.max_p99_ns = Some(value.parse().map_err(|_| "invalid p99 budget")?),
            }
        }
        if !(100..=100_000).contains(&config.samples)
            || config.warmup > 100_000
            || config.max_p99_ns == Some(0)
        {
            return Err(
                "samples must be 100..100000, warmup 0..100000, and budget positive".into(),
            );
        }
        Ok(config)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Distribution {
    pub count: usize,
    pub p50: u128,
    pub p95: u128,
    pub p99: u128,
    pub maximum: u128,
}
impl Distribution {
    pub fn from_samples(samples: &mut [u128]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        samples.sort_unstable();
        let at = |p: usize| samples[(samples.len() * p).div_ceil(100) - 1];
        Some(Self {
            count: samples.len(),
            p50: at(50),
            p95: at(95),
            p99: at(99),
            maximum: samples[samples.len() - 1],
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config(args: &[&str]) -> Result<Config, String> {
        Config::parse(args.iter().map(|s| s.to_string()))
    }
    #[test]
    fn profile_is_bounded_and_no_unspecified_budget_is_invented() {
        assert_eq!(config(&[]).unwrap().max_p99_ns, None);
        assert_eq!(
            config(&["--samples", "100", "--warmup", "0", "--max-p99-ns", "1"]).unwrap(),
            Config {
                samples: 100,
                warmup: 0,
                max_p99_ns: Some(1)
            }
        );
        for args in [
            vec!["--samples", "99"],
            vec!["--samples", "100001"],
            vec!["--warmup", "100001"],
            vec!["--max-p99-ns", "0"],
            vec!["--samples", "-1"],
            vec!["--samples", "+100"],
            vec!["--warmup", "1.0"],
            vec!["--samples"],
            vec!["--unknown", "1"],
            vec!["--samples", "100", "--samples", "100"],
        ] {
            assert!(config(&args).is_err());
        }
    }
    #[test]
    fn nearest_rank_tail_and_max_preserve_the_full_sample_population() {
        let mut samples: Vec<_> = (1..=100).rev().collect();
        assert_eq!(
            Distribution::from_samples(&mut samples),
            Some(Distribution {
                count: 100,
                p50: 50,
                p95: 95,
                p99: 99,
                maximum: 100,
            })
        );
        assert_eq!(Distribution::from_samples(&mut []), None);
        assert_eq!(Distribution::from_samples(&mut [7]).unwrap().p99, 7);
        assert_eq!(
            Distribution::from_samples(&mut [0; 100]).unwrap().maximum,
            0
        );
    }

    #[test]
    fn explicit_budget_accepts_equality_and_refuses_a_slow_tail_without_filtering_samples() {
        let d = Distribution::from_samples(&mut [10; 100]).unwrap();
        assert!(config(&[]).unwrap().accepts_total(d));
        assert!(config(&["--max-p99-ns", "10"]).unwrap().accepts_total(d));
        assert!(!config(&["--max-p99-ns", "9"]).unwrap().accepts_total(d));
    }
}
