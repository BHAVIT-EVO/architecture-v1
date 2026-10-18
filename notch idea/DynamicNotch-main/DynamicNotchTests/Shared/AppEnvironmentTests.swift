import XCTest
@testable import DynamicNotch

@MainActor
final class AppEnvironmentTests: XCTestCase {
    func testAppEnvironmentDetectsRunningInTestEnvironment() {
        XCTAssertTrue(AppEnvironment.isRunningTests)
        XCTAssertTrue(AppEnvironment.isRunningUnitTests)
    }

    func testAppContainerInitializesInactiveServicesInTestEnvironment() {
        let container = AppContainer()
        XCTAssertTrue(container.bluetoothViewModel.deviceName == "Unknown")
        XCTAssertFalse(container.bluetoothViewModel.isConnected)
    }
}
