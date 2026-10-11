import Foundation
import CoreGraphics
@main
final class NativeWindowFinder {
    private static let applicationName: String = "SWARX"

    static func main() -> Void {
        let records: [[String: Any]] = windows()
        let identity: Int? = selectedIdentity(records)
        printIdentity(identity)
    }

    private static func windows() -> [[String: Any]] {
        return CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    }

    private static func selectedIdentity(_ records: [[String: Any]]) -> Int? {
        return records.compactMap(identity).first
    }

    private static func identity(_ record: [String: Any]) -> Int? {
        let owner: String? = record[kCGWindowOwnerName as String] as? String
        let identifier: Int? = record[kCGWindowNumber as String] as? Int
        return matchingIdentity(owner, identifier)
    }

    private static func matchingIdentity(_ owner: String?, _ identifier: Int?) -> Int? {
        guard owner == applicationName else { return nil }
        return identifier
    }

    private static func printIdentity(_ identity: Int?) -> Void {
        guard let identity: Int = identity else { return }
        print(identity)
    }
}
