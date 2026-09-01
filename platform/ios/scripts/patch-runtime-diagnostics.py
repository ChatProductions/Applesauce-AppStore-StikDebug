from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    s = p.read_text()
    if old not in s:
        raise SystemExit(f"diagnostic patch needle not found in {path}: {old[:80]!r}")
    if s.count(old) != 1:
        raise SystemExit(f"diagnostic patch needle not unique in {path}: count={s.count(old)}")
    p.write_text(s.replace(old, new, 1))


# We know the previous device logs stop at the final libstdc++ patch. Put a
# marker at the exact return boundary of Dyld::do_initial_linking().
replace_once(
    "src/dyld.rs",
    """            patch_string_s_construct_null(dylib, mem);\n        }\n    }\n""",
    """            patch_string_s_construct_null(dylib, mem);\n        }\n        log!(\"LC-DIAG 10: Dyld::do_initial_linking completed\");\n    }\n""",
)

# Bracket the transition from dyld into Dynarmic construction. If 10/20 are
# present but 21 is absent, the host dies inside Cpu::new / Dynarmic setup.
replace_once(
    "src/environment.rs",
    """        let mut dyld = dyld::Dyld::new();\n        dyld.do_initial_linking(&bundle, &bins, &mut mem, &mut objc);\n\n        let cpu = cpu::Cpu::new(match options.direct_memory_access {\n            true => Some(&mut mem),\n            false => None,\n        });\n""",
    """        let mut dyld = dyld::Dyld::new();\n        dyld.do_initial_linking(&bundle, &bins, &mut mem, &mut objc);\n        log!(\"LC-DIAG 20: returned from Dyld::do_initial_linking\");\n\n        log!(\"LC-DIAG 21: about to construct Cpu/Dynarmic\");\n        let cpu = cpu::Cpu::new(match options.direct_memory_access {\n            true => Some(&mut mem),\n            false => None,\n        });\n        log!(\"LC-DIAG 22: Cpu/Dynarmic constructed successfully\");\n""",
)

# Bracket the Rust -> C++ FFI constructor itself.
replace_once(
    "src/cpu.rs",
    """        let dynarmic_wrapper =\n            unsafe { touchHLE_DynarmicWrapper_new(direct_memory_access_ptr, null_page_count) };\n        Cpu {\n""",
    """        log!(\"LC-DIAG 30: entering touchHLE_DynarmicWrapper_new\");\n        let dynarmic_wrapper =\n            unsafe { touchHLE_DynarmicWrapper_new(direct_memory_access_ptr, null_page_count) };\n        log!(\"LC-DIAG 31: touchHLE_DynarmicWrapper_new returned ptr={:?}\", dynarmic_wrapper);\n        Cpu {\n""",
)

# Bracket Dynarmic's actual JIT constructor. stderr is redirected to the same
# touchhle-host log by the iOS host, and fflush makes the last marker durable
# even if the next instruction traps or aborts.
replace_once(
    "src/cpu/dynarmic_wrapper/lib.cpp",
    """  DynarmicWrapper(void *direct_memory_access_ptr, size_t null_page_count) {\n    Dynarmic::A32::UserConfig user_config;\n""",
    """  DynarmicWrapper(void *direct_memory_access_ptr, size_t null_page_count) {\n    std::fprintf(stderr, \"LC-DIAG 40: DynarmicWrapper constructor entered\\n\");\n    std::fflush(stderr);\n    Dynarmic::A32::UserConfig user_config;\n""",
)
replace_once(
    "src/cpu/dynarmic_wrapper/lib.cpp",
    """    cpu = std::make_unique<Dynarmic::A32::Jit>(user_config);\n    env.cpu = cpu.get();\n""",
    """    std::fprintf(stderr, \"LC-DIAG 41: about to construct Dynarmic::A32::Jit\\n\");\n    std::fflush(stderr);\n    cpu = std::make_unique<Dynarmic::A32::Jit>(user_config);\n    std::fprintf(stderr, \"LC-DIAG 42: Dynarmic::A32::Jit constructed\\n\");\n    std::fflush(stderr);\n    env.cpu = cpu.get();\n""",
)

# The first cpu->Run() is where Dynarmic compiles/executes the first guest
# block. A marker before and after it distinguishes JIT construction from the
# first executable-code allocation/translation path.
replace_once(
    "src/cpu/dynarmic_wrapper/lib.cpp",
    """    if (ticks) {\n      env.ticks_remaining = *ticks;\n      hr = cpu->Run();\n    } else {\n      hr = cpu->Step();\n    }\n""",
    """    static bool first_execution = true;\n    if (first_execution) {\n      std::fprintf(stderr, \"LC-DIAG 50: entering first Dynarmic guest execution\\n\");\n      std::fflush(stderr);\n    }\n    if (ticks) {\n      env.ticks_remaining = *ticks;\n      hr = cpu->Run();\n    } else {\n      hr = cpu->Step();\n    }\n    if (first_execution) {\n      std::fprintf(stderr, \"LC-DIAG 51: first Dynarmic execution returned, halt=%u\\n\", unsigned(hr));\n      std::fflush(stderr);\n      first_execution = false;\n    }\n""",
)

print("Runtime diagnostic patch OK")
