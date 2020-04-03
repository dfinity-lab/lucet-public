use lucet_runtime_tests::globals_tests;

cfg_if::cfg_if! {
    if #[cfg(feature = "uffd")] {
        globals_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        globals_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
