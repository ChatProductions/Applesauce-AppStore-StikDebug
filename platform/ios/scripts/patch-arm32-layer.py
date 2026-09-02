from pathlib import Path

path = Path("platform/ios/Sources/main.m")
s = path.read_text()

old = r'''int main(int argc, char *argv[]) {
    (void)argc;
    (void)argv;

    redirect_diagnostics();
    touchhle_ios_log_jit_status("launch");

    char *base_path = SDL_GetBasePath();
    if (base_path != NULL) {
        chdir(base_path);
        SDL_free(base_path);
    }

    start_native_host();
    return 0;
}
'''

new = r'''static int32_t run_livecontainer_arm32_guest(int argc, char *argv[]) {
    if (argc < 2 || argv[0] == NULL || argv[1] == NULL) {
        return -1;
    }

    NSString *translatorExecutable = [NSString stringWithUTF8String:argv[0]];
    NSString *guestExecutable = [NSString stringWithUTF8String:argv[1]];
    if (translatorExecutable.length == 0 || guestExecutable.length == 0) {
        fprintf(stderr, "ARM32 layer: invalid argv paths\n");
        return 2;
    }

    // LiveContainer launches a 32-bit translation layer as:
    //   argv[0] = translation layer executable
    //   argv[1] = original 32-bit guest executable
    // NSBundle.mainBundle has already been swapped to the guest by LC, so do
    // NOT use Bundle.main to locate our own Frameworks. Derive our .app path
    // from argv[0] instead.
    NSString *translatorBundle = [translatorExecutable stringByDeletingLastPathComponent];
    NSString *guestBundle = [guestExecutable stringByDeletingLastPathComponent];
    NSString *corePath = [[translatorBundle stringByAppendingPathComponent:@"Frameworks"]
        stringByAppendingPathComponent:@"libhyperhle_core.dylib"];

    fprintf(stderr, "ARM32 layer: translator=%s\n", translatorBundle.fileSystemRepresentation);
    fprintf(stderr, "ARM32 layer: guest=%s\n", guestBundle.fileSystemRepresentation);
    fprintf(stderr, "ARM32 layer: core=%s\n", corePath.fileSystemRepresentation);

    if (chdir(translatorBundle.fileSystemRepresentation) != 0) {
        fprintf(stderr, "ARM32 layer: chdir failed: %s\n", strerror(errno));
        return 3;
    }

    // HyperHLE is Applesauce's broader-compatibility default core. The normal
    // UI still remains available when the app is launched directly rather
    // than invoked as LiveContainer's translation layer.
    setenv("TOUCHHLE_DEFAULT_OPTIONS_FILE", "touchHLE_default_options.txt", 1);

    void *handle = dlopen(corePath.fileSystemRepresentation, RTLD_NOW | RTLD_LOCAL);
    if (handle == NULL) {
        fprintf(stderr, "ARM32 layer: failed to load core: %s\n", dlerror());
        return 4;
    }

    TouchHLEIOSRunGameFn run_game = (TouchHLEIOSRunGameFn)dlsym(handle, "touchhle_ios_run_game");
    if (run_game == NULL) {
        fprintf(stderr, "ARM32 layer: touchhle_ios_run_game missing: %s\n", dlerror());
        return 5;
    }

    // Match the guest's declared orientation when possible.
    int32_t orientation = 0; // Portrait
    NSString *infoPath = [guestBundle stringByAppendingPathComponent:@"Info.plist"];
    NSDictionary *guestInfo = [NSDictionary dictionaryWithContentsOfFile:infoPath];
    NSArray *supported = guestInfo[@"UISupportedInterfaceOrientations"];
    if (![supported isKindOfClass:NSArray.class]) {
        supported = guestInfo[@"UISupportedInterfaceOrientations~ipad"];
    }
    if ([supported containsObject:@"UIInterfaceOrientationLandscapeLeft"]) {
        orientation = 1;
    } else if ([supported containsObject:@"UIInterfaceOrientationLandscapeRight"]) {
        orientation = 2;
    }

    touchhle_ios_log_jit_status("arm32-layer");
    fprintf(stderr, "ARM32 layer: starting translated guest\n");
    return touchhle_ios_launch_game(
        run_game,
        guestBundle.fileSystemRepresentation,
        1,
        orientation,
        0,
        1
    );
}

int main(int argc, char *argv[]) {
    redirect_diagnostics();
    touchhle_ios_log_jit_status("launch");

    // LiveContainer's 32-bit translation contract passes the original guest
    // executable as argv[1]. When present, bypass the Applesauce library UI
    // and run that guest immediately through HyperHLE/touchHLE.
    if (argc >= 2 && argv[1] != NULL && argv[1][0] == '/') {
        int32_t result = run_livecontainer_arm32_guest(argc, argv);
        if (result != -1) {
            return result;
        }
    }

    char *base_path = SDL_GetBasePath();
    if (base_path != NULL) {
        chdir(base_path);
        SDL_free(base_path);
    }

    start_native_host();
    return 0;
}
'''

if old not in s:
    raise SystemExit("main.m entrypoint pattern not found")

s = s.replace(old, new, 1)

# strerror() is used by the translation entrypoint.
if "#include <string.h>" not in s:
    s = s.replace("#include <stdint.h>\n", "#include <stdint.h>\n#include <string.h>\n", 1)

path.write_text(s)
print("ARM32 translation entrypoint patch OK")
