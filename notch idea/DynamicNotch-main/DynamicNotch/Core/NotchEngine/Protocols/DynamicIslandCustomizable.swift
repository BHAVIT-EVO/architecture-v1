//
//  DynamicIslandCustomizable.swift
//  DynamicNotch
//
//  Created by Евгений Петрукович on 5/30/26.
//

import Foundation

/// Protocol for customizing geometry when rendered as Apple's floating Dynamic Island pill.
///
/// In Apple's design, Dynamic Island is a floating pill detached from the screen bezel.
/// Conforming types provide specific pill dimensions and corner radii for both
/// **Compact** (`compactLeading`/`compactTrailing`) and **Expanded** (`expanded`) presentation modes.
protocol DynamicIslandCustomizable {
    /// Apple Dynamic Island: Frame size for compact presentation in floating pill mode.
    func compactDynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize

    /// Apple Dynamic Island: Frame size for expanded presentation in floating pill mode.
    func expandedDynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize

    /// Apple Dynamic Island: Corner radius for compact floating pill mode.
    func compactDynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat

    /// Apple Dynamic Island: Corner radius for expanded floating pill mode.
    func expandedDynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat

    // MARK: - Legacy Signatures
    func dynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize
    func dynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat
}

extension DynamicIslandCustomizable where Self: NotchContentProtocol {
    func compactDynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        dynamicIslandSize(baseWidth: baseWidth, baseHeight: baseHeight)
    }

    func dynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        compactSize(baseWidth: baseWidth, baseHeight: baseHeight)
    }

    func expandedDynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        let base = expandedSize(baseWidth: baseWidth, baseHeight: baseHeight)
        return CGSize(width: base.width + 40, height: base.height)
    }

    func compactDynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat {
        dynamicIslandCornerRadius(baseHeight: baseHeight)
    }

    func dynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat {
        baseHeight * 0.5
    }

    func expandedDynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat {
        baseHeight * 0.2
    }
}
