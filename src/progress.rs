use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressBarWrapper {
    bar: ProgressBar,
}

impl ProgressBarWrapper {
    pub fn new(total_size: u64, existing_size: u64) -> Self {
        let bar = if total_size > 0 {
            ProgressBar::new(total_size)
        } else {
            ProgressBar::new_spinner()
        };

        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .expect("valid template")
                .progress_chars("━▸ "),
        );

        if existing_size > 0 {
            bar.set_position(existing_size);
        }

        Self { bar }
    }

    pub fn inc(&self, amount: u64) {
        self.bar.inc(amount);
    }

    pub fn finish(&self) {
        self.bar.finish();
    }
}
