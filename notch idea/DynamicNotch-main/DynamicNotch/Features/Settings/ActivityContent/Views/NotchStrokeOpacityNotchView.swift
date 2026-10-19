//
//  NotchStrokeOpacityNotchView.swift
//  DynamicNotch
//

import SwiftUI

struct NotchStrokeOpacityNotchView: View {
    @ObservedObject var settingsViewModel: SettingsViewModel
    @Environment(\.notchScale) private var scale
    @Environment(\.isNotchlessScreen) private var isNotchlessScreen

    private var opacityPercentage: Int {
        Int(((settingsViewModel.application.notchStrokeOpacity / ApplicationSettingsStore.maxNotchStrokeOpacity) * 100).rounded())
    }

    var body: some View {
        HStack {
            Image(systemName: "inset.filled.capsule")
                .font(.system(size: isNotchlessScreen ? 16 : 18))
                .foregroundColor(.white)
            
            Spacer()
            
            AnimatedLevelText(
                level: opacityPercentage,
                fontSize: isNotchlessScreen ? 13 : 14,
                suffix: "%"
            )
        }
        .padding(.leading, isNotchlessScreen ? 4.scaled(by: scale) : 14.scaled(by: scale))
        .padding(.trailing, isNotchlessScreen ? 6.scaled(by: scale) : 14.scaled(by: scale))
    }
}
