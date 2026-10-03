import AppKit
import Foundation

@main
@MainActor
final class NativeIconGenerator {
    private static let sizes: [Int] = [16, 32, 64, 128, 256, 512, 1024]
    private static let argumentIndex: Int = 1
    private static let canvasSize: CGFloat = 128
    private static let center: CGFloat = 64
    private static let artworkScale: CGFloat = 0.84
    private static let sampleBits: Int = 8
    private static let sampleCount: Int = 4
    private static let backgroundRadius: CGFloat = 28
    private static let backgroundFrame: NSRect = NSRect(x: 4, y: 4, width: 120, height: 120)
    private static let backgroundColor: NSColor = NSColor(calibratedRed: 0.14, green: 0.17, blue: 0.29, alpha: 1)
    private static let branchColor: NSColor = NSColor(calibratedRed: 0.65, green: 0.69, blue: 1, alpha: 1)
    private static let activityColor: NSColor = NSColor(calibratedRed: 0.88, green: 0.91, blue: 1, alpha: 1)
    private static let branchWidth: CGFloat = 7
    private static let activityWidth: CGFloat = 4
    private static let rootPoint: NSPoint = NSPoint(x: 39, y: 91)
    private static let upperBranch: (NSPoint, NSPoint, NSPoint) = (
        NSPoint(x: 85, y: 62), NSPoint(x: 65, y: 89), NSPoint(x: 57, y: 62)
    )
    private static let lowerBranch: (NSPoint, NSPoint, NSPoint) = (
        NSPoint(x: 85, y: 34), NSPoint(x: 48, y: 69), NSPoint(x: 39, y: 40)
    )
    private static let nodes: [(CGFloat, CGFloat, CGFloat, NSColor)] = [
        (36, 93, 12, NSColor(calibratedRed: 0.61, green: 0.75, blue: 1, alpha: 1)),
        (89, 63, 10, NSColor(calibratedRed: 0.70, green: 0.63, blue: 1, alpha: 1)),
        (89, 34, 10, NSColor(calibratedRed: 0.55, green: 0.72, blue: 0.93, alpha: 1))
    ]
    private static let activityStart: NSPoint = NSPoint(x: 21, y: 37)
    private static let activityPoints: [NSPoint] = [
        NSPoint(x: 30, y: 37), NSPoint(x: 35, y: 50), NSPoint(x: 43, y: 27),
        NSPoint(x: 50, y: 41), NSPoint(x: 60, y: 41)
    ]

    static func main() throws -> Void {
        let destination: URL = destinationURL(CommandLine.arguments)
        try writeImages(destination)
    }

    static func writeIcon(size: Int, destination: URL) throws -> Void {
        let bitmap: NSBitmapImageRep = imageBitmap(size)
        let context: NSGraphicsContext = imageContext(bitmap)
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = context
        scaleCanvas(context.cgContext, size: size)
        drawArtwork()
        NSGraphicsContext.restoreGraphicsState()
        let bytes: Data = imageBytes(bitmap)
        let path: URL = destination.appendingPathComponent("icon-\(size).png")
        try bytes.write(to: path)
    }

    static func drawArtwork() -> Void {
        drawBackground()
        drawBranches()
        drawNodes()
        drawActivity()
    }

    private static func destinationURL(_ arguments: [String]) -> URL {
        guard arguments.count > argumentIndex else {
            fatalError("An icon destination directory is required.")
        }
        return URL(fileURLWithPath: arguments[argumentIndex])
    }

    private static func writeImages(_ destination: URL) throws -> Void {
        for size: Int in sizes {
            try writeIcon(size: size, destination: destination)
        }
    }

    private static func imageBitmap(_ size: Int) -> NSBitmapImageRep {
        let bitmap: NSBitmapImageRep? = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
            bitsPerSample: sampleBits, samplesPerPixel: sampleCount,
            hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
            bytesPerRow: 0, bitsPerPixel: 0
        )
        guard let bitmap: NSBitmapImageRep = bitmap else {
            fatalError("The icon bitmap could not be created.")
        }
        return bitmap
    }

    private static func imageContext(_ bitmap: NSBitmapImageRep) -> NSGraphicsContext {
        guard let context: NSGraphicsContext = NSGraphicsContext(bitmapImageRep: bitmap) else {
            fatalError("The icon graphics context could not be created.")
        }
        return context
    }

    private static func scaleCanvas(_ context: CGContext, size: Int) -> Void {
        let scale: CGFloat = CGFloat(size) / canvasSize
        context.scaleBy(x: scale, y: scale)
        context.translateBy(x: center, y: center)
        context.scaleBy(x: artworkScale, y: artworkScale)
        context.translateBy(x: -center, y: -center)
    }

    private static func drawBackground() -> Void {
        backgroundColor.setFill()
        let background: NSBezierPath = NSBezierPath(
            roundedRect: backgroundFrame, xRadius: backgroundRadius, yRadius: backgroundRadius
        )
        background.fill()
    }

    private static func drawBranches() -> Void {
        let path: NSBezierPath = NSBezierPath()
        path.move(to: rootPoint)
        path.curve(to: upperBranch.0, controlPoint1: upperBranch.1, controlPoint2: upperBranch.2)
        path.move(to: rootPoint)
        path.curve(to: lowerBranch.0, controlPoint1: lowerBranch.1, controlPoint2: lowerBranch.2)
        path.lineWidth = branchWidth
        path.lineCapStyle = .round
        branchColor.setStroke()
        path.stroke()
    }

    private static func drawNodes() -> Void {
        for node: (CGFloat, CGFloat, CGFloat, NSColor) in nodes {
            node.3.setFill()
            let frame: NSRect = NSRect(x: node.0 - node.2, y: node.1 - node.2,
                                      width: node.2 + node.2, height: node.2 + node.2)
            NSBezierPath(ovalIn: frame).fill()
        }
    }

    private static func drawActivity() -> Void {
        let path: NSBezierPath = NSBezierPath()
        path.move(to: activityStart)
        appendActivity(path)
        path.lineWidth = activityWidth
        path.lineCapStyle = .round
        path.lineJoinStyle = .round
        activityColor.setStroke()
        path.stroke()
    }

    private static func appendActivity(_ path: NSBezierPath) -> Void {
        for point: NSPoint in activityPoints {
            path.line(to: point)
        }
    }

    private static func imageBytes(_ bitmap: NSBitmapImageRep) -> Data {
        guard let data: Data = bitmap.representation(using: .png, properties: [:]) else {
            fatalError("The icon PNG could not be encoded.")
        }
        return data
    }
}
