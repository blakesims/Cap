use std::time::Duration;
use sysinfo::System;

pub struct ExportBufferConfig {
    pub rendered_frame_buffer: usize,
    pub encoder_input_buffer: usize,
    pub send_timeout: Duration,
}

impl ExportBufferConfig {
    pub fn for_current_system() -> Self {
        let total_ram_gb = get_total_memory_gb();

        let (rendered, encoder) = if total_ram_gb >= 32.0 {
            tracing::info!(
                total_ram_gb = %format!("{:.1}", total_ram_gb),
                rendered_buffer = 64,
                encoder_buffer = 32,
                "Using large buffer sizes for high-memory system"
            );
            (64, 32)
        } else if total_ram_gb >= 16.0 {
            tracing::info!(
                total_ram_gb = %format!("{:.1}", total_ram_gb),
                rendered_buffer = 32,
                encoder_buffer = 16,
                "Using medium buffer sizes for mid-memory system"
            );
            (32, 16)
        } else if total_ram_gb >= 8.0 {
            tracing::info!(
                total_ram_gb = %format!("{:.1}", total_ram_gb),
                rendered_buffer = 16,
                encoder_buffer = 8,
                "Using small buffer sizes for low-memory system"
            );
            (16, 8)
        } else {
            tracing::warn!(
                total_ram_gb = %format!("{:.1}", total_ram_gb),
                rendered_buffer = 8,
                encoder_buffer = 4,
                "Using minimal buffer sizes for very low-memory system"
            );
            (8, 4)
        };

        Self {
            rendered_frame_buffer: rendered,
            encoder_input_buffer: encoder,
            send_timeout: Duration::from_secs(5),
        }
    }
}

fn get_total_memory_gb() -> f64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0)
}
