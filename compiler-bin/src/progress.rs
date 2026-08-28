use std::fmt;
use std::io::{self, IsTerminal};
use std::time::{Duration, Instant};

use console::{Color, Style};
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};

const CARGO_PROGRESS_REGION_WIDTH: usize = 50;
const CARGO_PROGRESS_FIXED_OVERHEAD: usize = 17;
const SHIMMER_FRAME_INTERVAL: Duration = Duration::from_millis(80);

pub(crate) fn bar(total: usize, phase: &'static str, show: bool) -> ProgressBar {
    let progress = if show { ProgressBar::new(total as u64) } else { ProgressBar::hidden() };
    configure(&progress, phase);
    progress
}

pub(crate) fn phase(
    progress: &MultiProgress,
    total: usize,
    phase: &'static str,
    show: bool,
) -> ProgressBar {
    if !show {
        return ProgressBar::hidden();
    }
    let phase_progress = progress.add(ProgressBar::new(total as u64));
    configure(&phase_progress, phase);
    phase_progress
}

pub(crate) fn set_message(progress: &ProgressBar, message: &str) {
    progress.set_message(message.to_string());
}

pub(crate) fn finish(progress: &ProgressBar) {
    progress.set_message("");
    progress.finish();
}

pub(crate) fn report_completion(elapsed: Duration) {
    if !io::stderr().is_terminal() {
        return;
    }

    let label = format!("{:>12}", "Finished");
    let style = Style::new().green().bold();
    let jobs = rayon::current_num_threads();
    let job_label = if jobs == 1 { "job" } else { "jobs" };
    eprintln!("{} in {elapsed:.2?} via {jobs} {job_label}", style.apply_to(label));
}

fn configure(progress: &ProgressBar, phase: &'static str) {
    let started = Instant::now();
    let characters = phase.chars().collect::<Vec<_>>();
    let cycle = characters.len() + 4;
    let total = progress.length().unwrap_or_default();
    let count_width = total.to_string().len().max(4);
    let statistics_width = count_width * 2 + 2;
    let bar_width = CARGO_PROGRESS_REGION_WIDTH
        .saturating_sub(CARGO_PROGRESS_FIXED_OVERHEAD + statistics_width)
        .max(1);
    let phase_text = move |state: &ProgressState, writer: &mut dyn fmt::Write| {
        if state.is_finished() {
            let style = Style::new().fg(Color::TrueColor(255, 255, 255));
            write!(writer, "{}", style.apply_to(phase))
                .expect("writing to a formatter cannot fail");
            return;
        }

        let frame =
            (started.elapsed().as_millis() / SHIMMER_FRAME_INTERVAL.as_millis()) as usize % cycle;
        let highlight = frame as isize - 2;
        for (index, character) in characters.iter().enumerate() {
            let distance = (index as isize - highlight).unsigned_abs();
            let intensity = match distance {
                0 => 255,
                1 => 210,
                2 => 155,
                3 => 115,
                _ => 90,
            };
            let style = Style::new().fg(Color::TrueColor(intensity, intensity, intensity));
            write!(writer, "{}", style.apply_to(character))
                .expect("writing to a formatter cannot fail");
        }
    };
    let template =
        format!("{{phase:>12}} [{{bar:{bar_width}.cyan/blue}}] {{pos:>4}}/{{len:4}} {{msg}}");
    let style = ProgressStyle::with_template(&template)
        .expect("progress bar template is valid")
        .with_key("phase", phase_text)
        .progress_chars("=> ");
    progress.set_style(style);
    progress.enable_steady_tick(SHIMMER_FRAME_INTERVAL);
}
