//
//  NotchSoftwareUpdateEventsHandler.swift
//  DynamicNotch
//

import SwiftUI

@MainActor
final class NotchSoftwareUpdateEventsHandler {
    private let notchViewModel: NotchViewModel
    private let settingsViewModel: SettingsViewModel

    init(
        notchViewModel: NotchViewModel,
        settingsViewModel: SettingsViewModel
    ) {
        self.notchViewModel = notchViewModel
        self.settingsViewModel = settingsViewModel
    }

    func handleSoftwareUpdate(_ event: SoftwareUpdateEvent) {
        switch event {
        case .updateAvailable:
            let content = SoftwareUpdateNotchContent(settingsViewModel: settingsViewModel)
            notchViewModel.send(.showLiveActivity(content))
        case .upToDate:
            notchViewModel.send(.hideLiveActivity(id: NotchContentRegistry.SoftwareUpdate.update.id))
        }
    }
}
