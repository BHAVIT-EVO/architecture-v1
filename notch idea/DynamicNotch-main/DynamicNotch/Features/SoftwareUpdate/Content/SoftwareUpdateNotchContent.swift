//
//  SoftwareUpdateNotchContent.swift
//  DynamicNotch
//
//  Created by Евгений Петрукович on 7/17/26.
//

import SwiftUI

struct SoftwareUpdateNotchContent: NotchContentProtocol, DynamicIslandCustomizable {
    let id = NotchContentRegistry.SoftwareUpdate.update.id
    let settingsViewModel: SettingsViewModel?
    
    var priority: Int { NotchContentRegistry.SoftwareUpdate.update.priority }
    var isExpandable: Bool { true }

    init(settingsViewModel: SettingsViewModel? = nil) {
        self.settingsViewModel = settingsViewModel
    }
    
    func size(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        return .init(width: baseWidth + 70, height: baseHeight)
    }

    func dynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        .init(width: baseWidth + 60, height: baseHeight)
    }
    
    func expandedSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        .init(width: baseWidth + 120, height: baseHeight + 65)
    }

    func expandedDynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        .init(width: baseWidth + 180, height: baseHeight + 60)
    }
    
    func expandedCornerRadius(baseRadius: CGFloat) -> (top: CGFloat, bottom: CGFloat) {
        (top: 24, bottom: 34)
    }
    
    func expandedDynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat {
        baseHeight * 0.5
    }
    
    var windowLink: (@MainActor () -> Void)? {
        return {
            SettingsWindowController.shared.showWindow()
            NotificationCenter.default.post(
                name: NSNotification.Name("SelectSettingsSubPage"),
                object: SettingsSubPage.softwareUpdate
            )
        }
    }
    
    @MainActor
    func makeView() -> AnyView {
        AnyView(SoftwareUpdateNotchView())
    }
    
    @MainActor
    func makeExpandedView() -> AnyView {
        AnyView(SoftwareUpdateExpandedNotchView())
    }
}
