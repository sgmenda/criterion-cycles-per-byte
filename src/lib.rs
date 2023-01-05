//! `CyclesPerByte` measures clock cycles using the `rdtsc` instruction on x86
//! and x86_64 and the `cntfrq` instruction on aarch64.
//!
//! ```rust
//! # fn fibonacci_slow(_: usize) {}
//! # fn fibonacci_fast(_: usize) {}
//! use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
//! use criterion_cycles_per_byte::CyclesPerByte;
//!
//! fn bench(c: &mut Criterion<CyclesPerByte>) {
//!     let mut group = c.benchmark_group("fibonacci");
//!
//!     for i in 0..20 {
//!         group.bench_function(BenchmarkId::new("slow", i), |b| b.iter(|| fibonacci_slow(i)));
//!         group.bench_function(BenchmarkId::new("fast", i), |b| b.iter(|| fibonacci_fast(i)));
//!     }
//!
//!     group.finish()
//! }
//!
//! criterion_group!(
//!     name = my_bench;
//!     config = Criterion::default().with_measurement(CyclesPerByte);
//!     targets = bench
//! );
//! criterion_main!(my_bench);
//! ```

use criterion::{
    measurement::{Measurement, ValueFormatter},
    Throughput,
};
#[cfg(target_arch = "aarch64")]
use std::arch::asm;

#[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
compile_error!("criterion-cycles-per-byte currently relies on x86, x86_64, or aarch64.");

/// `CyclesPerByte` measures clock cycles using the `rdtsc` instruction on x86
/// and x86_64 and the `cntfrq` instruction on aarch64. `cpb` is the preferred
/// measurement for cryptographic algorithms.
pub struct CyclesPerByte;

// WARN: does not check for the cpu feature; but we'd panic anyway so...
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        core::arch::x86::_rdtsc()
    }
}

#[cfg(target_arch = "aarch64")]
fn cntfrq() -> u64 {
    // Adapted from https://github.com/google/benchmark/blob/1bd8098d3d5b7aa8e305e57b2451ab8f98a58965/src/cycleclock.h#L141-L148
    // h/t https://users.rust-lang.org/t/portable-way-to-measure-time-without-calling-the-os/44974
    let virtual_timer_value: u64;
    unsafe {
        asm!("mrs {}, cntvct_el0", out(reg) virtual_timer_value);
    }
    virtual_timer_value
}

fn now() -> u64{
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        return rdtsc();
#[cfg(target_arch = "aarch64")]
        return cntfrq();
}

impl Measurement for CyclesPerByte {
    type Intermediate = u64;
    type Value = u64;

    fn start(&self) -> Self::Intermediate {
        now()
    }

    fn end(&self, i: Self::Intermediate) -> Self::Value {
        now().saturating_sub(i)
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        v1 + v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, value: &Self::Value) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &CyclesPerByteFormatter
    }
}

struct CyclesPerByteFormatter;

impl ValueFormatter for CyclesPerByteFormatter {
    fn format_value(&self, value: f64) -> String {
        format!("{:.4} cycles", value)
    }

    fn format_throughput(&self, throughput: &Throughput, value: f64) -> String {
        match throughput {
            Throughput::Bytes(b) => format!("{:.4} cpb", value / *b as f64),
            Throughput::Elements(b) => format!("{:.4} cycles/{}", value, b),
            Throughput::BytesDecimal(b) => format!("{:.4} cpb (decimal)", value / *b as f64),
        }
    }

    fn scale_values(&self, _typical_value: f64, _values: &mut [f64]) -> &'static str {
        "cycles"
    }

    fn scale_throughputs(
        &self,
        _typical_value: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> &'static str {
        match throughput {
            Throughput::Bytes(n) => {
                for val in values {
                    *val /= *n as f64;
                }
                "cpb"
            }
            Throughput::Elements(n) => {
                for val in values {
                    *val /= *n as f64;
                }
                "c/e"
            }
            Throughput::BytesDecimal(n) => {
                for val in values {
                    *val /= *n as f64;
                }
                "cpb (decimal)"
            },
        }
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "cycles"
    }
}
