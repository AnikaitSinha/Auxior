use std::fs;
use std::time::Instant;

use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Flex, SparklineGraph, Text};
use crossterm::style::Color;

#[derive(Clone, Copy, Default)]
struct CpuTimes {
    idle: u64,
    total: u64,
}

fn read_per_core_times() -> Vec<CpuTimes> {
    let Ok(contents) = fs::read_to_string("/proc/stat") else {
        return Vec::new();
    };

    contents
        .lines()
        .filter(|line| {
            line.starts_with("cpu") && line.as_bytes().get(3).is_some_and(|b| b.is_ascii_digit())
        })
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let _name = parts.next()?;
            let values: Vec<u64> = parts.filter_map(|v| v.parse().ok()).collect();
            if values.len() < 4 {
                return None;
            }
            // user nice system idle iowait irq softirq steal ...
            let idle = values[3] + values.get(4).copied().unwrap_or(0);
            let total: u64 = values.iter().sum();
            Some(CpuTimes { idle, total })
        })
        .collect()
}

fn core_usage(prev: &CpuTimes, next: &CpuTimes) -> f32 {
    let total_d = next.total.saturating_sub(prev.total);
    let idle_d = next.idle.saturating_sub(prev.idle);
    if total_d == 0 {
        return 0.0;
    }
    ((1.0 - idle_d as f32 / total_d as f32) * 100.0).clamp(0.0, 100.0)
}

fn columns_for(
    cores: usize,
    available_rows: usize,
    available_width: usize,
    min_col_width: usize,
) -> usize {
    if cores == 0 || available_rows == 0 {
        return 1;
    }
    let by_height = cores.div_ceil(available_rows).max(1);
    let gap = 2;
    let by_width = if min_col_width == 0 {
        by_height
    } else {
        available_width
            .saturating_add(gap)
            .saturating_div(min_col_width.saturating_add(gap))
            .max(1)
    };
    by_height.min(by_width).max(1)
}

fn trim(history: &mut Vec<f32>, window: usize) {
    let keep = window.saturating_mul(2).max(window);
    if history.len() > keep {
        let drop = history.len() - keep;
        history.drain(..drop);
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(AppConfig::new().target_fps(10))?;
    let window = 60;

    let mut prev = read_per_core_times();
    let mut histories: Vec<Vec<f32>> = prev.iter().map(|_| Vec::new()).collect();
    let mut last_sample = Instant::now();

    app.run(move |buf, _events| {
        let now = Instant::now();
        if now.duration_since(last_sample).as_millis() >= 200 {
            let next = read_per_core_times();
            if next.len() != histories.len() {
                histories = next.iter().map(|_| Vec::new()).collect();
                prev = next;
            } else {
                for (i, (p, n)) in prev.iter().zip(next.iter()).enumerate() {
                    let usage = core_usage(p, n);
                    histories[i].push(usage);
                    trim(&mut histories[i], window);
                }
                prev = next;
            }
            last_sample = now;
        }

        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let cores = histories.len().max(1);
        // Outer chrome ≈ 5 rows; inner cores panel has its own border (+2).
        let available_rows = area.height.saturating_sub(7).max(1) as usize;
        // Inner panel width after outer border+padding and inner border.
        let available_width = area.width.saturating_sub(6).max(1) as usize;
        let min_col_width = 6 + 8 + 5 + 2; // label + bar + status + gaps
        let cols = columns_for(cores, available_rows, available_width, min_col_width);
        let rows_per_col = cores.div_ceil(cols);

        let low = Color::Rgb {
            r: 40,
            g: 200,
            b: 90,
        };
        let high = Color::Rgb {
            r: 230,
            g: 50,
            b: 50,
        };

        let mut columns = Flex::row().gap(2).flex(1);
        for col in 0..cols {
            let start = col * rows_per_col;
            let end = ((col + 1) * rows_per_col).min(cores);
            let mut column = Flex::column().gap(0).flex(1);

            for core_idx in start..end {
                let history = histories.get(core_idx).map(Vec::as_slice).unwrap_or(&[]);
                column = column.child(
                    SparklineGraph::new()
                        .window(window)
                        .range(0.0, 100.0)
                        .color_steps(3)
                        .min_len_label(5)
                        .min_len_status(4)
                        .label(Text::new(format!("CPU{core_idx:<2}")))
                        .start_color(low)
                        .end_color(high)
                        .values(history.iter().copied()),
                );
            }

            columns = columns.child(column);
        }

        let panel_h = rows_per_col as u16 + 2; // content rows + border
        let cores_panel = Div::new()
            .border(true)
            .title(Text::new("Cores").fg(Color::DarkGrey))
            .height(panel_h)
            .child(columns);

        let avg = histories
            .iter()
            .filter_map(|h| h.last().copied())
            .sum::<f32>()
            / cores.max(1) as f32;

        Div::new()
            .border(true)
            .title(
                Text::new(format!(
                    "CPU  ·  {cores}c  ·  avg {avg:5.1}%  ·  {cols} col(s)  ·  q quit"
                ))
                .fg(Color::Cyan)
                .bold(true),
            )
            .padding(1)
            .child(cores_panel)
            .render(&mut canvas);

        ControlFlow::Continue
    })?;

    Ok(())
}
