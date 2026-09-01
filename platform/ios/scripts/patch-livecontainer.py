from pathlib import Path
import re

p = Path("platform/ios/Sources/NativeHost.swift")
s = p.read_text()

# LiveContainer owns JIT. Applesauce only checks whether the host process
# already has JIT and must never invoke StikDebug itself.
s, n = re.subn(
    r"    static var current: JITMethod \{\n.*?\n    \}\n\n    /// Shown when a launch is held back because JIT is off\.",
    '''    static var current: JITMethod {
        if touchhle_ios_jit_available() {
            return .permanent
        }
        return .external
    }

    /// Shown when a launch is held back because JIT is off.''',
    s,
    count=1,
    flags=re.S,
)
if n != 1:
    raise SystemExit(f"JITMethod.current patch count={n}")

s, n = re.subn(
    r"    var unavailableMessage: String \{\n.*?\n    \}\n\n    /// Explains what has to happen, and how often\.",
    '''    var unavailableMessage: String {
        return "JIT is not active in this LiveContainer session. Return to LiveContainer, "
            + "enable Launch with JIT for Applesauce LC, and relaunch it."
    }

    /// Explains what has to happen, and how often.''',
    s,
    count=1,
    flags=re.S,
)
if n != 1:
    raise SystemExit(f"unavailableMessage patch count={n}")

s, n = re.subn(
    r"    var footer: String \{\n.*?\n    \}\n\}\n\nprivate struct EnableJITButton",
    '''    var footer: String {
        if touchhle_ios_jit_available() {
            return "JIT is active for this LiveContainer session."
        }
        return "JIT is supplied by LiveContainer before Applesauce starts."
    }
}

private struct EnableJITButton''',
    s,
    count=1,
    flags=re.S,
)
if n != 1:
    raise SystemExit(f"footer patch count={n}")

# Remove the in-app JIT action from the launch warning.
s, n = re.subn(
    r'\n\s*Button\("Enable JIT"\) \{\n\s*if let url = StikDebug\.enableJITURL \{\n\s*openURL\(url\)\n\s*\}\n\s*\}\n',
    '\n',
    s,
    count=1,
)
if n != 1:
    raise SystemExit(f"held Enable JIT removal count={n}")

# Replace the Settings JIT action with a read-only status row.
s, n = re.subn(
    r'''                Section \{\n\s*EnableJITButton\(\)\n\s*\} header: \{\n\s*Text\("JIT"\)\n\s*\} footer: \{\n\s*Text\(JITMethod\.current\.footer\)\n\s*\}''',
    '''                Section {
                    HStack {
                        Label("LiveContainer JIT", systemImage: touchhle_ios_jit_available() ? "checkmark.circle.fill" : "xmark.circle")
                        Spacer()
                        Text(touchhle_ios_jit_available() ? "Active" : "Not Active")
                            .foregroundStyle(.secondary)
                    }
                } header: {
                    Text("JIT")
                } footer: {
                    Text(JITMethod.current.footer)
                }''',
    s,
    count=1,
)
if n != 1:
    raise SystemExit(f"Settings JIT section patch count={n}")

# Add access to the native diagnostic logs already written by main.m.
needle = '''            } footer: {
                Text("Displays a small frame-rate counter beside the exit button. This is intended for testing and is off by default.")
            }
'''
if needle not in s:
    raise SystemExit("Developer Tools insertion point not found")
s = s.replace(
    needle,
    needle + '''
            Section("Diagnostics") {
                NavigationLink {
                    LCDiagnosticsView()
                } label: {
                    Label("Runtime Log", systemImage: "doc.text.magnifyingglass")
                }
            }
''',
    1,
)

marker = '\nprivate struct AboutView: View {\n'
if marker not in s:
    raise SystemExit("AboutView insertion point not found")

diagnostics = r'''
private struct LCDiagnosticsView: View {
    @State private var text = "Loading…"
    @State private var showingPrevious = true

    private var documents: URL {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    }

    private func load() {
        let name = showingPrevious ? "touchhle-host-previous.log" : "touchhle-host.log"
        let url = documents.appendingPathComponent(name)
        text = (try? String(contentsOf: url, encoding: .utf8))
            ?? "No log file found for this session."
    }

    var body: some View {
        VStack(spacing: 0) {
            Picker("Log", selection: $showingPrevious) {
                Text("Previous Session").tag(true)
                Text("Current Session").tag(false)
            }
            .pickerStyle(.segmented)
            .padding()
            .onChange(of: showingPrevious) { _ in load() }

            ScrollView {
                Text(text)
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding()
            }

            Button {
                UIPasteboard.general.string = text
            } label: {
                Label("Copy Log", systemImage: "doc.on.doc")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .padding()
        }
        .navigationTitle("Runtime Log")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { load() }
    }
}

'''
s = s.replace(marker, '\n' + diagnostics + 'private struct AboutView: View {\n', 1)

p.write_text(s)

# Hard assertions so a changed upstream file cannot silently create a bad IPA.
out = p.read_text()
current = out.split('static var current: JITMethod', 1)[1].split('/// Shown when a launch', 1)[0]
assert 'return .stikDebug' not in current
assert 'Button("Enable JIT")' not in out
assert 'Runtime Log' in out
assert 'touchhle-host-previous.log' in out
print("LiveContainer patch OK")
