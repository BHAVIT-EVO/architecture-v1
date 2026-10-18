// Isolated AppKit fixture for opt-in local AX verification. No capture,
// personal-history reads, network, or existing application control.
// Compile into a temporary .app bundle; pass one fixture directory argument.
import AppKit
import Foundation

final class FixtureDelegate: NSObject, NSApplicationDelegate {
    private var windows: [NSWindow] = []

    func applicationDidFinishLaunching(_ notification: Notification) {
        guard CommandLine.arguments.count == 2 else {
            NSApp.terminate(nil)
            return
        }
        let directory = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
        let application = NSRunningApplication.current
        guard let launch = application.launchDate else {
            try? Data("launchDate unavailable".utf8).write(to: directory.appendingPathComponent("error.txt"))
            NSApp.terminate(nil)
            return
        }
        for index in 1...2 {
            let document = directory.appendingPathComponent("fixture-\(index).txt")
            let window = NSWindow(
                contentRect: NSRect(x: 180 + index * 35, y: 220 + index * 35, width: 500, height: 220),
                styleMask: [.titled, .closable, .miniaturizable], backing: .buffered, defer: false
            )
            window.title = "Evo isolated restore fixture"
            window.setAccessibilityIdentifier("evo.fixture.\(index)")
            window.setAccessibilityDocument(document.absoluteString)
            window.isReleasedWhenClosed = false
            let label = NSTextField(labelWithString: "Synthetic document \(index). No personal data.\nThis fixture closes itself after three minutes.")
            label.frame = NSRect(x: 24, y: 80, width: 460, height: 70)
            window.contentView?.addSubview(label)
            window.makeKeyAndOrderFront(nil)
            windows.append(window)
        }
        NSApp.activate(ignoringOtherApps: true)
        let manifest: [String: Any] = [
            "pid": application.processIdentifier,
            "process_started_at": String(format: "%016llx", launch.timeIntervalSince1970.bitPattern),
            "window_identifier": "evo.fixture.1",
            "document": directory.appendingPathComponent("fixture-1.txt").path
        ]
        if let data = try? JSONSerialization.data(withJSONObject: manifest, options: [.sortedKeys]) {
            try? data.write(to: directory.appendingPathComponent("manifest.json"), options: .atomic)
        }
        Timer.scheduledTimer(withTimeInterval: 180, repeats: false) { _ in NSApp.terminate(nil) }
    }
}

let application = NSApplication.shared
let delegate = FixtureDelegate()
application.delegate = delegate
application.setActivationPolicy(.regular)
application.run()
