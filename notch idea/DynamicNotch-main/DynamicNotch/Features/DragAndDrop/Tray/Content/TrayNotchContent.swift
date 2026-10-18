//
//  TrayContent.swift
//  DynamicNotch
//
//  Created by Евгений Петрукович on 4/25/26.
//

import SwiftUI

enum TrayEvent {
    case dragStarted
    case dragEnded
    case dropped
}

struct TrayNotchContent: NotchContentProtocol, DynamicIslandCustomizable {
    let id = NotchContentRegistry.DragAndDrop.tray.id

    let airDropViewModel: AirDropNotchViewModel
    let settingsViewModel: SettingsViewModel
    
    var priority: Int { NotchContentRegistry.DragAndDrop.tray.priority }

    var strokeColor: Color {
        DragAndDropTarget.tray.activityStrokeColor
    }
    
    func cornerRadius(baseRadius: CGFloat) -> (top: CGFloat, bottom: CGFloat) {
        return (top: 20, bottom: 26)
    }
    
    func size(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        return .init(width: baseWidth + 40, height: baseHeight + 110)
    }
    
    func dynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat {
        baseHeight * 0.2
    }
    
    @MainActor
    func makeView() -> AnyView {
        AnyView(
            TrayNotchView(
                airDropViewModel: airDropViewModel
            )
        )
    }
}
