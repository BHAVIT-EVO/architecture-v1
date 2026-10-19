import Foundation
import Combine

final class InactiveBluetoothService: BluetoothServiceProtocol, @unchecked Sendable {
    var lastConnectedDevice: BluetoothAudioDevice? { nil }
    var connectedDevices: [BluetoothAudioDevice] { [] }

    var lastConnectedDevicePublisher: AnyPublisher<BluetoothAudioDevice?, Never> {
        Empty().eraseToAnyPublisher()
    }

    var connectedDevicesPublisher: AnyPublisher<[BluetoothAudioDevice], Never> {
        Empty().eraseToAnyPublisher()
    }

    var deviceConnectedEventPublisher: AnyPublisher<BluetoothAudioDevice, Never> {
        Empty().eraseToAnyPublisher()
    }

    func refreshConnectedDeviceBatteries() {}
}
