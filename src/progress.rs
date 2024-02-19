use std::borrow::Cow;

use duration_macro::duration;
use indicatif::{ProgressBar, ProgressStyle};
use rustic_core::{Progress, ProgressBars};

#[derive(Clone, Debug)]
pub struct SproutProgressBar {}
#[derive(Clone, Debug)]
pub struct SproutProgress {
    pub bar: ProgressBar,
}

impl SproutProgress {
    pub fn new() -> Self {
        Self {
            bar: ProgressBar::new(1024),
        }
    }

    pub fn hidden() -> Self {
        Self {
            bar: ProgressBar::hidden(),
        }
    }

    pub fn spinner() -> Self {
        Self {
            bar: ProgressBar::new_spinner(),
        }
    }
}

impl ProgressBars for SproutProgressBar {
    type P = SproutProgress;

    fn progress_hidden(&self) -> Self::P {
        SproutProgress::hidden()
    }

    fn progress_spinner(&self, prefix: impl Into<Cow<'static, str>>) -> Self::P {
        let p = SproutProgress::spinner();

        p.bar.set_message(prefix);

        p.bar
            .set_style(ProgressStyle::with_template("{spinner:^9.green} {msg}").unwrap());

        p.bar.enable_steady_tick(duration!(100 ms));

        p
    }

    fn progress_counter(&self, _prefix: impl Into<Cow<'static, str>>) -> Self::P {
        let p = SproutProgress::new();
        p.bar.set_style(
            ProgressStyle::with_template(
                "{spinner:^9.green} [{elapsed_precise:}] {wide_bar:.green/cyan.dim} {pos:.bold}/{len:} ({eta:})",
            )
            .unwrap()
            .progress_chars("▰▶▱"),
        );

        p
    }

    fn progress_bytes(&self, _prefix: impl Into<Cow<'static, str>>) -> Self::P {
        let p = SproutProgress::new();
        p.bar.enable_steady_tick(duration!(100 ms));
        p.bar.set_style(ProgressStyle::with_template("{spinner:^9.green} [{elapsed_precise:}] {wide_bar:.green/cyan.dim} {bytes:.bold}/{total_bytes:} ({eta:})")
        .unwrap()
        .progress_chars("▰▶▱"));

        p
    }
}

impl Progress for SproutProgress {
    fn is_hidden(&self) -> bool {
        self.bar.is_hidden()
    }
    fn set_length(&self, len: u64) {
        self.bar.set_length(len)
    }
    fn set_title(&self, title: &'static str) {
        self.bar.set_message(title);
    }
    fn inc(&self, inc: u64) {
        self.bar.inc(inc);
    }
    fn finish(&self) {
        self.bar.finish_and_clear();
    }
}
