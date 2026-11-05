internal import AppKit
import SwiftUI
import QuartzCore

struct ScreenshotFlyAnimationView: View {
    let image: NSImage
    let startPoint: CGPoint
    let notchPoint: CGPoint
    let screenSize: CGSize
    let isNotchless: Bool
    let onReachNotch: (@MainActor () -> Void)?
    let onEarlyComplete: @MainActor () -> Void
    let onFinished: @MainActor () -> Void
    
    @State private var startTime: Date?
    @State private var hasTriggeredReachNotch = false
    @State private var hasTriggeredEarlyComplete = false
    @State private var hasFinished = false
    
    private let duration: Double = 0.44
    private let sliceCount: Int = 36
    
    private func easeFlightProgress(_ t: CGFloat) -> CGFloat {
        if t < 0.40 {
            return 1.4 * t * t
        } else {
            let p = (t - 0.40) / 0.60
            return 0.224 + 0.776 * pow(p, 1.45)
        }
    }
    
    var body: some View {
        TimelineView(.animation) { timelineContext in
            let elapsed = timelineContext.date.timeIntervalSince(startTime ?? timelineContext.date)
            let rawProgress = min(1.0, max(0.0, elapsed / duration))
            let progress = easeFlightProgress(rawProgress)
            
            ZStack(alignment: .top) {
                Canvas { context, size in
                    let trajectory = ScreenshotFlyTrajectory(
                        start: startPoint,
                        notch: notchPoint,
                        screenSize: screenSize
                    )
                    
                    let imgSize = image.size
                    let aspect = max(0.5, min(2.5, (imgSize.width > 0 && imgSize.height > 0) ? (imgSize.width / imgSize.height) : 1.0))
                    let baseWidth = min(360.0, max(240.0, screenSize.width * 0.24))
                    let baseHeight = baseWidth / aspect
                    
                    let scale = max(0.18, 1.0 - 0.75 * progress)
                    let alpha = progress < 0.72 ? 1.0 : max(0.0, 1.0 - (progress - 0.72) / 0.28)
                    let deltaT = max(0.06, 0.18 * (1.0 - 0.65 * progress))
                    let totalHeight = max(10.0, baseHeight * scale)
                    
                    var spinePoints: [CGPoint] = []
                    var tangents: [CGFloat] = []
                    var leftBoundary: [CGPoint] = []
                    var rightBoundary: [CGPoint] = []
                    var sliceWidths: [CGFloat] = []
                    
                    let bowMultiplier: CGFloat = trajectory.isLateral ? (trajectory.deltaX * 0.045 * (1.0 - progress) * sin(progress * .pi)) : 0.0
                    
                    for i in 0..<sliceCount {
                        let v = CGFloat(i) / CGFloat(sliceCount - 1)
                        let t = max(0.001, min(0.999, progress + (0.5 - v) * deltaT))
                        
                        var pt = trajectory.point(at: t)
                        let theta = trajectory.tangentAngle(at: t)
                        let normal = CGPoint(x: -sin(theta), y: cos(theta))
                        
                        if bowMultiplier != 0 {
                            let bow = sin(v * .pi) * bowMultiplier
                            pt.x += normal.x * bow
                            pt.y += normal.y * bow
                        }
                        let taper = 1.0 - (1.0 - v) * (0.42 * progress)
                        let w = baseWidth * scale * taper
                        
                        let halfW = w / 2.0
                        let leftPt = CGPoint(x: pt.x - normal.x * halfW, y: pt.y - normal.y * halfW)
                        let rightPt = CGPoint(x: pt.x + normal.x * halfW, y: pt.y + normal.y * halfW)
                        
                        spinePoints.append(pt)
                        tangents.append(theta)
                        leftBoundary.append(leftPt)
                        rightBoundary.append(rightPt)
                        sliceWidths.append(w)
                    }
                    
                    var boundaryPath = Path()
                    if let firstLeft = leftBoundary.first {
                        boundaryPath.move(to: firstLeft)
                        for pt in leftBoundary.dropFirst() {
                            boundaryPath.addLine(to: pt)
                        }
                        if let lastRight = rightBoundary.last {
                            boundaryPath.addLine(to: lastRight)
                        }
                        for pt in rightBoundary.dropLast().reversed() {
                            boundaryPath.addLine(to: pt)
                        }
                        boundaryPath.closeSubpath()
                    }
                    
                    var shadowContext = context
                    shadowContext.addFilter(.blur(radius: 14))
                    shadowContext.fill(boundaryPath, with: .color(.black.opacity(0.35 * alpha)))
                    
                    let resolvedImage = context.resolve(Image(nsImage: image))
                    let flightBlurRadius: CGFloat = 8.0 + 6.0 * progress
                    
                    context.drawLayer { layerCtx in
                        layerCtx.addFilter(.blur(radius: flightBlurRadius))
                        
                        for i in 0..<(sliceCount - 1) {
                            let vMid = (CGFloat(i) + 0.5) / CGFloat(sliceCount - 1)
                            let ptMid = CGPoint(
                                x: (spinePoints[i].x + spinePoints[i + 1].x) / 2.0,
                                y: (spinePoints[i].y + spinePoints[i + 1].y) / 2.0
                            )
                            let thetaMid = (tangents[i] + tangents[i + 1]) / 2.0
                            let wMid = (sliceWidths[i] + sliceWidths[i + 1]) / 2.0
                            
                            let dist = hypot(spinePoints[i + 1].x - spinePoints[i].x, spinePoints[i + 1].y - spinePoints[i].y)
                            let stripH = max(2.5, dist * 1.30)
                            
                            var sliceCtx = layerCtx
                            sliceCtx.opacity = alpha
                            sliceCtx.translateBy(x: ptMid.x, y: ptMid.y)
                            sliceCtx.rotate(by: Angle(radians: thetaMid + .pi / 2))
                            
                            let clipRect = CGRect(x: -wMid * 0.51, y: -stripH * 0.55, width: wMid * 1.02, height: stripH * 1.10)
                            sliceCtx.clip(to: Path(clipRect))
                            
                            let imgRect = CGRect(x: -wMid / 2.0, y: -vMid * totalHeight, width: wMid, height: totalHeight)
                            sliceCtx.draw(resolvedImage, in: imgRect)
                        }
                    }
                    
                    if progress > 0.45 && !spinePoints.isEmpty {
                        let glowOpacity = min(0.35, (progress - 0.45) * 0.9) * alpha
                        var glowCtx = context
                        glowCtx.addFilter(.blur(radius: 12))
                        let frontPt = spinePoints[0]
                        let glowRect = CGRect(x: frontPt.x - 45, y: frontPt.y - 15, width: 90, height: 30)
                        glowCtx.fill(Path(ellipseIn: glowRect), with: .color(.white.opacity(glowOpacity)))
                    }
                }
                .onAppear {
                    if startTime == nil {
                        startTime = timelineContext.date
                    }
                }
                .onChange(of: rawProgress) { _, newProgress in
                    if newProgress >= 0.70 && !hasTriggeredReachNotch {
                        hasTriggeredReachNotch = true
                        onReachNotch?()
                    }
                    if newProgress >= 0.78 && !hasTriggeredEarlyComplete {
                        hasTriggeredEarlyComplete = true
                        onEarlyComplete()
                    }
                    if newProgress >= 1.0 && !hasFinished {
                        hasFinished = true
                        onFinished()
                    }
                }
            }
            .ignoresSafeArea()
        }
    }
    
    private struct ScreenshotFlyTrajectory {
        let p0: CGPoint
        let p1: CGPoint
        let p2: CGPoint
        let p3: CGPoint
        let isLateral: Bool
        let deltaX: CGFloat
        
        init(start: CGPoint, notch: CGPoint, screenSize: CGSize) {
            let deltaX = start.x - notch.x
            let deltaY = start.y - notch.y
            let isLateral = abs(deltaX) > 40
            self.isLateral = isLateral
            self.deltaX = deltaX
            
            self.p0 = start
            self.p3 = notch
            
            if isLateral {
                let liftRatio: CGFloat = min(0.55, max(0.35, 0.45 * (abs(deltaY) / max(screenSize.height, 1))))
                let p1X = start.x - deltaX * 0.12
                let p1Y = start.y - deltaY * liftRatio
                self.p1 = CGPoint(x: p1X, y: p1Y)
                
                let p2X = notch.x + deltaX * 0.05
                let p2Y = notch.y + deltaY * 0.16
                self.p2 = CGPoint(x: p2X, y: p2Y)
            } else {
                self.p1 = CGPoint(x: notch.x, y: start.y - deltaY * 0.35)
                self.p2 = CGPoint(x: notch.x, y: start.y - deltaY * 0.70)
            }
        }
        
        func point(at t: CGFloat) -> CGPoint {
            let clampedT = max(0.0, min(1.0, t))
            let u = 1.0 - clampedT
            let tt = clampedT * clampedT
            let uu = u * u
            let uuu = uu * u
            let ttt = tt * clampedT
            
            let x = uuu * p0.x + 3 * uu * clampedT * p1.x + 3 * u * tt * p2.x + ttt * p3.x
            let y = uuu * p0.y + 3 * uu * clampedT * p1.y + 3 * u * tt * p2.y + ttt * p3.y
            return CGPoint(x: x, y: y)
        }
        
        func derivative(at t: CGFloat) -> CGPoint {
            let clampedT = max(0.0001, min(0.9999, t))
            let u = 1.0 - clampedT
            let tt = clampedT * clampedT
            let uu = u * u
            
            let dx = 3 * uu * (p1.x - p0.x) + 6 * u * clampedT * (p2.x - p1.x) + 3 * tt * (p3.x - p2.x)
            let dy = 3 * uu * (p1.y - p0.y) + 6 * u * clampedT * (p2.y - p1.y) + 3 * tt * (p3.y - p2.y)
            return CGPoint(x: dx, y: dy)
        }
        
        func tangentAngle(at t: CGFloat) -> CGFloat {
            let d = derivative(at: t)
            return atan2(d.y, d.x)
        }
    }
}

@MainActor
final class ScreenshotFlyAnimationService {
    static let shared = ScreenshotFlyAnimationService()
    
    private var activeWindow: NSPanel?
    private var completionHandler: (() -> Void)?
    private var fallbackTask: Task<Void, Never>?
    
    private init() {}
    
    func playFlyToNotchAnimation(
        image: NSImage,
        onReachNotch: (@MainActor () -> Void)? = nil,
        onComplete: @escaping () -> Void
    ) {
        guard let mainScreen = NSScreen.main else {
            onComplete()
            return
        }
        
        fallbackTask?.cancel()
        fallbackTask = nil
        activeWindow?.orderOut(nil)
        activeWindow = nil
        completionHandler = onComplete
        
        let mouseLoc = NSEvent.mouseLocation
        let targetScreen = NSScreen.screens.first { NSMouseInRect(mouseLoc, $0.frame, false) } ?? mainScreen
        let screenFrame = targetScreen.frame
        
        let isNotchless = targetScreen.isNotchless
        let notchY: CGFloat = isNotchless ? 4.0 : -12.0
        let notchPoint = CGPoint(x: screenFrame.width / 2.0, y: notchY)
        
        let rawStartX = mouseLoc.x - screenFrame.minX
        let rawStartY = screenFrame.maxY - mouseLoc.y
        let startX = max(70.0, min(screenFrame.width - 70.0, rawStartX))
        let startY = max(notchPoint.y + 90.0, min(screenFrame.height - 70.0, rawStartY))
        let startPoint = CGPoint(x: startX, y: startY)
        
        let panel = OverlayPanelFactory.makePanel(
            frame: screenFrame,
            level: .floating,
            isFloatingPanel: true
        )
        panel.ignoresMouseEvents = true
        panel.alphaValue = 1.0
        
        let flyView = ScreenshotFlyAnimationView(
            image: image,
            startPoint: startPoint,
            notchPoint: notchPoint,
            screenSize: screenFrame.size,
            isNotchless: isNotchless,
            onReachNotch: onReachNotch,
            onEarlyComplete: { [weak self] in
                self?.triggerEarlyCompletion()
            },
            onFinished: { [weak self, weak panel] in
                self?.cleanupPanel(panel)
            }
        )
        
        panel.contentView = NSHostingView(rootView: flyView)
        panel.orderFront(nil)
        
        self.activeWindow = panel
        self.fallbackTask = Task { @MainActor [weak self, weak panel] in
            try? await Task.sleep(for: .milliseconds(550))
            guard !Task.isCancelled else { return }
            self?.cleanupPanel(panel)
        }
    }
    
    private func triggerEarlyCompletion() {
        guard let completion = completionHandler else { return }
        completionHandler = nil
        completion()
    }
    
    private func cleanupPanel(_ panel: NSPanel?) {
        triggerEarlyCompletion()
        panel?.orderOut(nil)
        if activeWindow === panel {
            activeWindow = nil
        }
        fallbackTask?.cancel()
        fallbackTask = nil
    }
}
