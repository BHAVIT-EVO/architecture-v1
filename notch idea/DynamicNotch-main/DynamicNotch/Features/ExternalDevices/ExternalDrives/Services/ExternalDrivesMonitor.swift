import Foundation
internal import AppKit
import OSLog
import DiskArbitration
import IOKit

final class ExternalDrivesMonitor {

    private let logger = Logger(subsystem: Bundle.main.bundleIdentifier ?? "DynamicNotch", category: "ExternalDrivesMonitor")

    var onDriveEvent: ((ExternalDriveModel) -> Void)?

    private let daSession = DASessionCreate(kCFAllocatorDefault)
    private var observers: [NSObjectProtocol] = []
    private var knownVolumes: [String: (name: String, icon: NSImage?, isEjectable: Bool)] = [:]
    private let queue = DispatchQueue(label: "com.dynamicnotch.external-drives-monitor")

    deinit {
        stopMonitoring()
    }

    func startMonitoring() {
        guard observers.isEmpty else { return }

        let center = NSWorkspace.shared.notificationCenter

        let mountObserver = center.addObserver(
            forName: NSWorkspace.didMountNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.handleVolumeMounted(notification)
        }

        let unmountObserver = center.addObserver(
            forName: NSWorkspace.didUnmountNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.handleVolumeUnmounted(notification)
        }

        observers = [mountObserver, unmountObserver]
        logger.info("External drives monitoring started")
    }

    func stopMonitoring() {
        let center = NSWorkspace.shared.notificationCenter
        for observer in observers {
            center.removeObserver(observer)
        }
        observers.removeAll()
        logger.info("External drives monitoring stopped")
    }

    func ejectDrive(at url: URL, completion: ((Bool) -> Void)? = nil) {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            do {
                try NSWorkspace.shared.unmountAndEjectDevice(at: url)
                self?.logger.info("Successfully ejected drive at \(url.path)")
                DispatchQueue.main.async {
                    completion?(true)
                }
            } catch {
                self?.logger.error("Failed to eject drive at \(url.path): \(error.localizedDescription)")
                DispatchQueue.main.async {
                    completion?(false)
                }
            }
        }
    }

    func isExternalPortConnectedDevice(url: URL) -> Bool {
        guard url.isFileURL else { return false }

        // Fast check: reject internal volumes reported by file system resource values
        let resourceKeys: Set<URLResourceKey> = [
            .volumeIsInternalKey
        ]
        if let values = try? url.resourceValues(forKeys: resourceKeys), values.volumeIsInternal == true {
            return false
        }

        // Fast check: ignore standard system paths
        let path = url.path
        if path == "/" || path.hasPrefix("/System") || path.hasPrefix("/private") {
            return false
        }

        guard let session = daSession ?? DASessionCreate(kCFAllocatorDefault) else {
            return false
        }

        guard let disk = DADiskCreateFromVolumePath(kCFAllocatorDefault, session, url as CFURL) else {
            return false
        }

        guard let desc = DADiskCopyDescription(disk) as? [String: Any] else {
            return false
        }

        // 1. Must not be internal
        if let isInternal = desc[kDADiskDescriptionDeviceInternalKey as String] as? Bool, isInternal {
            return false
        }

        // 2. Must not be a network volume (SMB, AFP, NFS)
        if let isNetwork = desc[kDADiskDescriptionVolumeNetworkKey as String] as? Bool, isNetwork {
            return false
        }

        // 3. Must not be a disk image (DMG, ISO, sparse image)
        if let model = desc[kDADiskDescriptionDeviceModelKey as String] as? String,
           model.localizedCaseInsensitiveContains("Disk Image") {
            return false
        }

        if let protocolName = desc[kDADiskDescriptionDeviceProtocolKey as String] as? String {
            let lowerProto = protocolName.lowercased()
            if lowerProto.contains("virtual") || lowerProto.contains("disk image") || lowerProto.contains("apple fabric") {
                return false
            }
        }

        // 4. Verify physical interconnect via IOKit
        let media = DADiskCopyIOMedia(disk)
        guard media != IO_OBJECT_NULL else {
            return false
        }
        defer { IOObjectRelease(media) }

        var isExternalLocation = false
        var physicalInterconnect: String?

        // Check "Protocol Characteristics" in IOKit registry
        let characteristicsKey = "Protocol Characteristics" as CFString
        if let characteristicsRef = IORegistryEntrySearchCFProperty(
            media,
            kIOServicePlane,
            characteristicsKey,
            kCFAllocatorDefault,
            IOOptionBits(kIORegistryIterateParents | kIORegistryIterateRecursively)
        ) as? [String: Any] {
            if let location = characteristicsRef["Physical Interconnect Location"] as? String {
                if location.caseInsensitiveCompare("External") == .orderedSame {
                    isExternalLocation = true
                }
            }
            if let interconnect = characteristicsRef["Physical Interconnect"] as? String {
                physicalInterconnect = interconnect
            }
        }

        // Fallback: search directly for "Physical Interconnect Location"
        if !isExternalLocation {
            let locationKey = "Physical Interconnect Location" as CFString
            if let locationRef = IORegistryEntrySearchCFProperty(
                media,
                kIOServicePlane,
                locationKey,
                kCFAllocatorDefault,
                IOOptionBits(kIORegistryIterateParents | kIORegistryIterateRecursively)
            ) as? String {
                if locationRef.caseInsensitiveCompare("External") == .orderedSame {
                    isExternalLocation = true
                }
            }
        }

        // Must be physically connected externally
        guard isExternalLocation else {
            return false
        }

        // Ensure physical interconnect is not virtual, file-backed, or RAM
        if let physicalInterconnect {
            let lower = physicalInterconnect.lowercased()
            if lower.contains("virtual") || lower.contains("file") || lower.contains("ram") {
                return false
            }
        }

        return true
    }

    private func handleVolumeMounted(_ notification: Notification) {
        guard let url = notification.userInfo?[NSWorkspace.volumeURLUserInfoKey] as? URL else {
            return
        }

        // Register only external devices that connect externally through a port
        guard isExternalPortConnectedDevice(url: url) else {
            return
        }

        let keys: Set<URLResourceKey> = [
            .volumeNameKey,
            .volumeLocalizedNameKey,
            .volumeIsEjectableKey,
            .volumeTotalCapacityKey,
            .volumeAvailableCapacityKey
        ]

        guard let resourceValues = try? url.resourceValues(forKeys: keys) else {
            return
        }

        let isEjectable = resourceValues.volumeIsEjectable ?? false
        let name = resourceValues.volumeLocalizedName ?? resourceValues.volumeName ?? url.lastPathComponent
        let total = Int64(resourceValues.volumeTotalCapacity ?? 0)
        let free = Int64(resourceValues.volumeAvailableCapacity ?? 0)
        let icon = NSWorkspace.shared.icon(forFile: url.path)

        // Cache info for unmount notification
        knownVolumes[url.path] = (name: name, icon: icon, isEjectable: isEjectable)

        let model = ExternalDriveModel(
            id: url.path,
            name: name,
            volumeURL: url,
            totalBytes: total,
            freeBytes: free,
            isEjectable: isEjectable,
            eventType: .connected,
            icon: icon
        )

        logger.info("External drive mounted via port: \(name)")
        onDriveEvent?(model)
    }

    private func handleVolumeUnmounted(_ notification: Notification) {
        guard let url = notification.userInfo?[NSWorkspace.volumeURLUserInfoKey] as? URL else {
            return
        }

        guard let cached = knownVolumes.removeValue(forKey: url.path) else {
            return
        }

        guard cached.isEjectable else { return }

        let model = ExternalDriveModel(
            id: url.path,
            name: cached.name,
            volumeURL: nil,
            totalBytes: 0,
            freeBytes: 0,
            isEjectable: cached.isEjectable,
            eventType: .ejected,
            icon: cached.icon
        )

        logger.info("External drive safely unmounted: \(cached.name)")
        onDriveEvent?(model)
    }
}
