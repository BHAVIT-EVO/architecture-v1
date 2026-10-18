import SwiftUI

struct DynamicIslandShape: Shape {
    var cornerRadius: CGFloat

    var animatableData: CGFloat {
        get { cornerRadius }
        set { cornerRadius = newValue }
    }

    func path(in rect: CGRect) -> Path {
        var path = Path()
        guard rect.width > 0, rect.height > 0 else { return path }
        let maxRadius = min(rect.width, rect.height) * 0.5
        let clampedRadius = max(0, min(cornerRadius, maxRadius))
        path.addRoundedRect(in: rect, cornerSize: CGSize(width: clampedRadius, height: clampedRadius))
        return path
    }
}
