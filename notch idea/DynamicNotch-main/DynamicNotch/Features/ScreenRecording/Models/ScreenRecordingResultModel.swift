internal import AppKit
import AVFoundation

struct ScreenRecordingResultModel: Identifiable, Equatable {
    let id = UUID()
    let fileURL: URL
    let thumbnail: NSImage
    let fileName: String
    var formattedDuration: String
    let timestamp: Date

    init(
        fileURL: URL,
        thumbnail: NSImage,
        fileName: String,
        formattedDuration: String = "00:00",
        timestamp: Date = Date()
    ) {
        self.fileURL = fileURL
        self.thumbnail = thumbnail
        self.fileName = fileName
        self.formattedDuration = formattedDuration
        self.timestamp = timestamp
    }

    static func == (lhs: ScreenRecordingResultModel, rhs: ScreenRecordingResultModel) -> Bool {
        lhs.id == rhs.id && lhs.fileURL == rhs.fileURL
    }

    static func formatDuration(for url: URL) async -> String {
        let asset = AVURLAsset(url: url)
        do {
            let duration = try await asset.load(.duration)
            let seconds = duration.seconds
            guard seconds.isFinite && !seconds.isNaN && seconds > 0 else {
                return "00:00"
            }
            let totalSeconds = Int(seconds.rounded())
            let minutes = totalSeconds / 60
            let remainingSeconds = totalSeconds % 60
            return String(format: "%02d:%02d", minutes, remainingSeconds)
        } catch {
            return "00:00"
        }
    }
}

