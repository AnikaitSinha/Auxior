use std::fs;
use std::time::{Duration, Instant};

use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Flex, Text};
use crossterm::style::Color;

const TARGET_FPS: u64 = 60;
/// Linux USER_HZ; process CPU times in `/proc/self/stat` are in these ticks.
const CLOCK_TICKS: f32 = 100.0;

/// utime + stime from `/proc/self/stat` (fields 14 and 15).
fn read_process_cpu_ticks() -> Option<u64> {
    let contents = fs::read_to_string("/proc/self/stat").ok()?;
    let rest = contents.split(')').nth(1)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime.saturating_add(stime))
}

fn process_cpu_pct(prev_ticks: u64, next_ticks: u64, elapsed: Duration) -> f32 {
    let elapsed_secs = elapsed.as_secs_f32();
    if elapsed_secs <= f32::EPSILON {
        return 0.0;
    }
    let delta_ticks = next_ticks.saturating_sub(prev_ticks) as f32;
    let elapsed_ticks = elapsed_secs * CLOCK_TICKS;
    if elapsed_ticks <= f32::EPSILON {
        return 0.0;
    }
    (delta_ticks / elapsed_ticks) * 100.0
}

fn read_process_mem_kib() -> Option<(u64, u64)> {
    let contents = fs::read_to_string("/proc/self/status").ok()?;
    let mut rss = None;
    let mut size = None;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss = rest.split_whitespace().next()?.parse().ok();
        } else if let Some(rest) = line.strip_prefix("VmSize:") {
            size = rest.split_whitespace().next()?.parse().ok();
        }
        if rss.is_some() && size.is_some() {
            break;
        }
    }
    Some((rss?, size.unwrap_or(0)))
}

fn format_mib(kib: u64) -> String {
    format!("{:.1} MiB", kib as f64 / 1024.0)
}

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(AppConfig::new().target_fps(TARGET_FPS))?;

    let mut fps_window_start = Instant::now();
    let mut frames_in_window = 0_u64;
    let mut actual_fps = 0.0_f32;
    let mut last_frame = Instant::now();
    let mut frame_ms = 0.0_f32;

    let mut prev_cpu_ticks = read_process_cpu_ticks().unwrap_or(0);
    let mut last_cpu_sample = Instant::now();
    let mut cpu_pct = 0.0_f32;
    let mut last_stats = Instant::now();

    let mut rss_kib = 0_u64;
    let mut vsize_kib = 0_u64;

    app.run(move |buf, _events| {
        let frame_start = Instant::now();
        frames_in_window += 1;

        if last_stats.elapsed().as_millis() >= 250 {
            if let Some(next_ticks) = read_process_cpu_ticks() {
                cpu_pct = process_cpu_pct(prev_cpu_ticks, next_ticks, last_cpu_sample.elapsed());
                prev_cpu_ticks = next_ticks;
                last_cpu_sample = Instant::now();
            }
            if let Some((rss, vsize)) = read_process_mem_kib() {
                rss_kib = rss;
                vsize_kib = vsize;
            }
            last_stats = Instant::now();
        }

        if fps_window_start.elapsed().as_secs_f32() >= 1.0 {
            actual_fps = frames_in_window as f32 / fps_window_start.elapsed().as_secs_f32();
            frames_in_window = 0;
            fps_window_start = Instant::now();
        }

        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let panel_w = 34_u16;
        let panel_h = 12_u16;
        // Outer border(1) + padding(1) → content starts at (2, 2).
        let content_w = area.width.saturating_sub(4);
        let stats_x = content_w.saturating_sub(panel_w);
        let cells = area.width as u64 * area.height as u64;

        let stats = Flex::column()
            .gap(0)
            .child(Text::new(format!("target fps   {TARGET_FPS}")).fg(Color::DarkGrey))
            .child(Text::new(format!("actual fps   {actual_fps:5.1}")).fg(Color::Green))
            .child(Text::new(format!("frame        {frame_ms:5.2} ms")).fg(Color::Yellow))
            .child(Text::new(format!("proc cpu     {cpu_pct:5.1}%")).fg(Color::Magenta))
            .child(Text::new(format!("proc rss     {}", format_mib(rss_kib))).fg(Color::Cyan))
            .child(Text::new(format!("proc vsize   {}", format_mib(vsize_kib))).fg(Color::Cyan))
            .child(
                Text::new(format!(
                    "cells/frame  {cells}  ({}x{})",
                    area.width, area.height
                ))
                .fg(Color::DarkGrey),
            )
            .child(Text::new("q / Esc quit").fg(Color::DarkGrey));

        let stats_panel = Div::new()
            .border(true)
            .title(
                Text::new("perf")
                    .fg(Color::Rgb {
                        r: 120,
                        g: 220,
                        b: 255,
                    })
                    .bold(true),
            )
            .padding(1)
            .x(stats_x)
            .y(0)
            .width(panel_w)
            .height(panel_h)
            .child(stats);

        Div::new()
            .border(true)
            .padding(1)
            .child(stats_panel)
            .render(&mut canvas);

        frame_ms = last_frame.elapsed().as_secs_f32() * 1000.0;
        last_frame = frame_start;

        ControlFlow::Continue
    })?;

    Ok(())
}
