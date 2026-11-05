import Foundation

enum AppEnvironment: Sendable {
    /// Indicates whether the app is executing within any test environment (unit tests or UI tests).
    nonisolated static var isRunningTests: Bool {
        isRunningUITests || isRunningUnitTests
    }

    /// Indicates whether the app is running Xcode UI tests (passed via `-ui-testing` flag).
    nonisolated static var isRunningUITests: Bool {
        ProcessInfo.processInfo.arguments.contains("-ui-testing")
    }

    /// Indicates whether the app process is running as a test host for unit tests.
    nonisolated static var isRunningUnitTests: Bool {
        ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] != nil ||
        ProcessInfo.processInfo.environment["XCTestBundlePath"] != nil ||
        ProcessInfo.processInfo.environment["XCInjectBundleInto"] != nil ||
        NSClassFromString("XCTestCase") != nil
    }
}
