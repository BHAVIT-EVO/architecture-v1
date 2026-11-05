import SwiftUI

struct ScreenshotNotchContent: NotchContentProtocol, DynamicIslandCustomizable {
    let id = NotchContentRegistry.Screenshot.active.id
    let viewModel: ScreenshotViewModel
    
    var priority: Int { NotchContentRegistry.Screenshot.active.priority }
    
    func size(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        return .init(width: baseWidth + 160, height: baseHeight + 105)
    }
    
    func dynamicIslandSize(baseWidth: CGFloat, baseHeight: CGFloat) -> CGSize {
        return .init(width: baseWidth + 200, height: baseHeight + 100)
    }
    
    func cornerRadius(baseRadius: CGFloat) -> (top: CGFloat, bottom: CGFloat) {
        return (top: 28, bottom: 40)
    }
    
    func dynamicIslandCornerRadius(baseHeight: CGFloat) -> CGFloat {
        baseHeight * 0.3
    }
    
    @MainActor
    func makeView() -> AnyView {
        AnyView(ScreenshotNotchView(screenshotViewModel: viewModel))
    }
}
