import AppKit
import Foundation

@MainActor
final class NativeApplicationIcon {
    private static let iconName: String = "Observatory.icns"
    private static var loaded: Bool = false

    static func applyIcon() -> Void {
        let url: URL? = iconURL(Bundle.main.resourceURL)
        let image: NSImage? = iconImage(url)
        applyImage(image)
    }

    static func isLoaded() -> Bool {
        return loaded
    }

    private static func iconURL(_ resources: URL?) -> URL? {
        guard let resources: URL = resources else { return nil }
        return resources.appendingPathComponent(iconName)
    }

    private static func iconImage(_ url: URL?) -> NSImage? {
        guard let url: URL = url else { return nil }
        return NSImage(contentsOf: url)
    }

    private static func applyImage(_ image: NSImage?) -> Void {
        guard let image: NSImage = image else { return }
        NSApplication.shared.applicationIconImage = image
        loaded = true
    }
}
