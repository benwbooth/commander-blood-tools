# GPU Test Isolation

The 2026-09-08 parallel library run terminated with SIGSEGV. The saved core is
identified by PID 3911281, timestamp 2026-09-08 11:58:47 PDT, and executable
`target/debug/deps/commander_blood_game-3c6cac899d4a034d`.

Inspect the host evidence with `coredumpctl --no-pager info 3911281`.

The crashing thread 3912018 has this stack:

```text
libvulkan: loader_get_icd_and_device
libvulkan: terminator_SetDebugUtilsObjectNameEXT
libvulkan: SetDebugUtilsObjectNameEXT
ash: set_debug_utils_object_name
wgpu_hal: DeviceShared::set_object_name
wgpu_hal: create_pipeline_layout
wgpu_core: create_pipeline_layout_impl
commander_blood_game::render::tests::render_offscreen_artwork_layers
commander_blood_game::render::tests::srgb_artwork_and_overlay_match_every_cpu_expanded_dac_level
```

At the same instant thread 3912017 is in:

```text
ld-linux: munmap / _dl_unmap / _dl_close_worker
libc: dlclose
libvulkan: unload_drivers_without_physical_devices
libvulkan: vkEnumeratePhysicalDevices
wgpu_hal: enumerate_adapters
wgpu_core: request_adapter
commander_blood_game::render::tests::original_manu3_renders_nonblank_inside_wide_and_portrait_viewports
```

This identifies concurrent Vulkan adapter enumeration/driver unloading and
pipeline construction, not a COD localization failure. It does not establish
the loader's internal root cause. A subsequent unchanged parallel library run
passed 964 tests, and 30 unchanged focused render runs at 16 threads each passed
all 15 tests. Thus absence of the crash in a short rerun is weak evidence.

The test-only `gpu_test::lock` serializes complete GPU test lifetimes across
the render, bridge, alien, and ignored live-services tests. Acquire it before
creating any GPU owners; its local binding must outlive devices, queues, and
resources so their destruction occurs under the same guard. It recovers a
poisoned mutex so one failed assertion does not prevent later tests from running.
CPU tests remain parallel. No pixel assertions, backend flags, shader validation,
or rendering paths are disabled. The game executable is unchanged.

This is a harness mitigation of the observed concurrency, not a Vulkan fix or
proof of safe concurrent independent renderer initialization in production.
The runtime uses one renderer; concurrent renderer lifecycle behavior needs a
separate isolated stress test if it becomes a supported application use case.

Verification after the mitigation: the default-parallel game library passed
964 tests (43 ignored); 30 focused render runs at 16 test threads each passed
all 15 tests (450 checks). Game-package all-targets checking also passed. The
ignored live-services tests were compiled but not executed in this verification.
