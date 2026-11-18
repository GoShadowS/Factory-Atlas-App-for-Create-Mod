pub mod exporters;
pub mod fixtures;
pub mod jar_importer;
pub mod machine_speeds;
pub mod mc_model;
pub mod model;
pub mod nbt_parser;
pub mod project_format;
pub mod solver;
pub mod storage;
pub mod visual_matcher;
pub mod worker_pool;

/// Returns current process RSS memory in KiB on Linux and Windows using OS primitives.
pub fn current_process_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
        let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
        Some(pages * 4)
    }
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct ProcessMemoryCounters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set_size: usize,
            working_set_size: usize,
            quota_peak_paged_pool_usage: usize,
            quota_paged_pool_usage: usize,
            quota_peak_non_paged_pool_usage: usize,
            quota_non_paged_pool_usage: usize,
            pagefile_usage: usize,
            peak_pagefile_usage: usize,
        }

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
        }

        #[link(name = "psapi")]
        unsafe extern "system" {
            fn GetProcessMemoryInfo(
                process: *mut std::ffi::c_void,
                ppsmem_counters: *mut ProcessMemoryCounters,
                cb: u32,
            ) -> i32;
        }

        // SAFETY: `pmc` is zero-initialized with valid `cb` size before calling Win32 API.
        unsafe {
            let mut pmc = std::mem::zeroed::<ProcessMemoryCounters>();
            pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
            let ok = GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb);
            if ok != 0 {
                return Some((pmc.working_set_size / 1024) as u64);
            }
        }
        None
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}
