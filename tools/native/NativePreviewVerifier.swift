import AppKit
import Foundation
import WebKit

@MainActor
final class NativePreviewVerifier {
    private static let iconLoadedKey: String = "iconLoaded"
    private static let emptyText: String = ""
    private static let actionKey: String = "action"
    private static let nativeWindowKey: String = "nativeWindow"
    private static let snapshotKey: String = "snapshot"
    private static let widthKey: String = "width"
    private static let heightKey: String = "height"
    private static let labelKey: String = "label"
    private static let successKey: String = "success"
    private static let directoryKey: String = "directory"

    private static let enabledName: String = "OBSERVATORY_QA"
    private static let enabledValue: String = "1"
    private static let captureOnlyName: String = "OBSERVATORY_QA_CAPTURE_ONLY"
    private static let focusName: String = "OBSERVATORY_QA_FOCUS"
    private static let rootName: String = "OBSERVATORY_ROOT"
    private static let reportScript: String = "__NATIVE_REPORT_SCRIPT__"
    private static let operationSeconds: Double = 30
    private static let errorDomain: String = "Observatory.NativePreview"
    private static let maximumErrorCharacters: Int = 500
    private static let normalSize: NSSize = NSSize(width: 1240, height: 850)
    private static let fullSize: NSSize = NSSize(width: 1680, height: 850)
    private static let smallSize: NSSize = NSSize(width: 1000, height: 700)
    private static let resourceSteps: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] = [
        ("native-command-link-ready", "running", "What's running", "", normalSize, "command-linked"),
        ("native-command-resource-live-samples", "polls", "What's running", "", normalSize, "updates"),
        ("native-command-resources-wide", "running", "What's running", "", normalSize, "command-linked"),
        ("native-command-resources-small", "running", "What's running", "", smallSize, "command-linked"),
        ("native-command-resources-final", "running", "What's running", "", normalSize, "command-linked")
    ]
    private static let descendantSteps: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] = [
        ("native-descendant-running", "descendants", "What's running", "", normalSize, "descendant-linked"),
        ("native-descendant-refresh", "polls", "What's running", "", normalSize, "descendant-open-polls"),
        ("native-descendant-table", "owned-table", "Processes", "table", normalSize, "descendant-owner"),
        ("native-descendant-tree", "owned-tree", "Processes", "tree", normalSize, "descendant-owner"),
        ("native-descendant-runtime", "runtimes", "Runtimes", "", normalSize, "descendant-owner"),
        ("native-descendant-history", "history", "Resource history", "", normalSize, "descendant-owner"),
        ("native-descendant-small", "descendants", "What's running", "", smallSize, "descendant-linked"),
        ("native-descendant-tree-small", "owned-tree", "Processes", "tree", smallSize, "descendant-owner"),
        ("native-descendant-final", "descendants", "What's running", "", normalSize, "descendant-linked")
    ]
    private static let activityLayoutSteps: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] = [
        ("native-activity-four-columns", "activity", "Activity", "", fullSize, "activity-layout"),
        ("native-activity-two-columns", "activity", "Activity", "", normalSize, "activity-layout"),
        ("native-activity-one-column", "activity", "Activity", "", smallSize, "activity-layout")
    ]
    private static let connectorSteps: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] = [
        ("native-connector-61685", "connector-first", "Processes", "tree", normalSize, "tree"),
        ("native-connector-61639", "connector-second", "Processes", "tree", normalSize, "tree"),
        ("native-retained-action-history", "activity", "Activity", "", normalSize, "action-history"),
        ("native-restart-live-samples", "polls", "Activity", "", normalSize, "updates")
    ] + activityLayoutSteps
    private static let expiryAction: String = "current-expiry"
    private static let expiryNanoseconds: UInt64 = 130_000_000_000
    private static let currentWorkSteps: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] = [
        ("native-current-live", "running", "What's running", "", normalSize, "current-work-live"),
        ("native-current-expired", expiryAction, "What's running", "", normalSize, "current-work-hidden"),
        ("native-current-reappeared", "running", "What's running", "", smallSize, "current-work-live"),
        ("native-current-completed-history", "activity", "Activity", "", normalSize, "current-work-completed"),
        ("native-current-completed-hidden", "running", "What's running", "", normalSize, "current-work-hidden")
    ]
    private static let activeTaskSteps: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] = [
        ("native-task-only", "running", "What's running", "", normalSize, "current-task-only"),
        ("native-task-only-polls", "polls", "What's running", "", normalSize, "current-task-polls"),
        ("native-task-only-small", "running", "What's running", "", smallSize, "current-task-only"),
        ("native-task-command", "running", "What's running", "", normalSize, "current-work-live"),
        ("native-task-between-commands", "running", "What's running", "", smallSize, "current-task-only"),
        ("native-task-completed-history", "activity", "Activity", "", normalSize, "current-work-completed"),
        ("native-task-completed-hidden", "running", "What's running", "", normalSize, "current-work-hidden")
    ]
    private static let steps: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] = [
        ("native-action-running", "activity", "Activity", "", normalSize, "action-running"),
        ("native-action-elapsed-update", "polls", "Activity", "", normalSize, "action-progress"),
        ("native-command-resources", "running", "What's running", "", normalSize, "command-linked"),
        ("native-table-full", "table", "Processes", "table", fullSize, "table"),
        ("native-running-full", "running", "What's running", "", fullSize, ""),
        ("native-tree-full", "tree", "Processes", "tree", fullSize, "tree"),
        ("native-running-wide", "running", "What's running", "", normalSize, ""),
        ("native-table-wide", "table", "Processes", "table", normalSize, "table"),
        ("native-tree", "tree", "Processes", "tree", normalSize, "tree"),
        ("native-tree-zoom", "exercise", "Processes", "tree", normalSize, "zoom"),
        ("native-launch-updates", "polls", "Processes", "tree", normalSize, "polls"),
        ("native-tree-refresh", "polls", "Processes", "tree", normalSize, "polls"),
        ("native-history-wide", "history", "Resource history", "", normalSize, "history"),
        ("native-small", "running", "What's running", "", smallSize, ""),
        ("native-table-small", "table", "Processes", "table", smallSize, "table"),
        ("native-tree-small", "tree", "Processes", "tree", smallSize, "tree"),
        ("native-history-small", "history", "Resource history", "", smallSize, "history"),
        ("native-runtimes-small", "runtimes", "Runtimes", "", smallSize, ""),
        ("native-activity-small", "activity", "Activity", "", smallSize, ""),
        ("native-runtimes", "runtimes", "Runtimes", "", normalSize, ""),
        ("native-activity", "activity", "Activity", "", normalSize, ""),
        ("native-running-final", "running", "What's running", "", normalSize, ""),
        ("native-roster", "running", "What's running", "", normalSize, "logical"),
        ("native-action-history", "activity", "Activity", "", normalSize, "action-history")
    ] + activityLayoutSteps
    private static let focusedSelections: [String: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)]] = [
        "connectors": connectorSteps,
        "resources": resourceSteps,
        "descendants": descendantSteps,
        "current-work": currentWorkSteps,
        "active-task": activeTaskSteps
    ]
    private let view: WKWebView
    private let window: NSWindow
    private let directory: URL
    private var operationNumber: Int = 0
    private var finishedOperation: Int = 0
    private var currentLabel: String = "startup"
    private var currentStage: String = "directory"

    private init(context: (WKWebView, NSWindow, URL)) {
        view = context.0
        window = context.1
        directory = context.2.appendingPathComponent("native-qa-" + UUID().uuidString)
    }

    static func begin(view: WKWebView?, window: NSWindow?) -> Void {
        let enabled: Bool = ProcessInfo.processInfo.environment[enabledName] == enabledValue
        let directory: URL? = evidenceDirectory(ProcessInfo.processInfo.environment[rootName])
        let context: (WKWebView, NSWindow, URL)? = previewContext(view, window, directory)
        let selected: (WKWebView, NSWindow, URL)? = enabledContext(context, enabled)
        startPreview(selected)
    }

    private func start() -> Void {
        Task { @MainActor () -> Void in await self.run() }
    }

    private func run() async -> Void {
        let failure: Error? = await checkedRun()
        await captureFailure(failure)
        finishRun(failure)
    }

    private func captureFailure(_ error: Error?) async -> Void {
        guard error != nil else { return }
        do {
            let script: String = Self.reportScript + "\nreturn NativePreviewReport.read(iconLoaded);"
            let result: Any = try await operation { (completion: @escaping @MainActor @Sendable (Result<Any, Error>) -> Void) -> Void in
                self.view.callAsyncJavaScript(script, arguments: [Self.iconLoadedKey: NativeApplicationIcon.isLoaded()],
                                              in: nil, in: .page, completionHandler: completion)
            }
            try writeRecord(try reportDictionary(result), directory.appendingPathComponent("failure-visible-report.json"))
            let image: NSImage = try await snapshot()
            try imageData(image).write(to: directory.appendingPathComponent("failure-visible-window.png"), options: .atomic)
        } catch {
            print("Native failure capture: " + error.localizedDescription)
        }
    }

    private func checkedRun() async -> Error? {
        do {
            try await sequence()
            return nil
        } catch {
            return error
        }
    }

    private func sequence() async throws -> Void {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try await runSteps()
    }

    private func runSteps() async throws -> Void {
        for step in selectedSteps() {
            try await perform(step)
        }
    }

    private func selectedSteps() -> [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] {
        let captureOnly: Bool = ProcessInfo.processInfo.environment[Self.captureOnlyName] == Self.enabledValue
        if captureOnly {
            return [("native-roster-capture", "running", "What's running", Self.emptyText, view.bounds.size, "updates")]
        }
        return focusedSteps()
    }

    private func focusedSteps() -> [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] {
        let focus: String? = ProcessInfo.processInfo.environment[Self.focusName]
        let name: String = selectedFocusName(focus)
        let selection: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)]? = Self.focusedSelections[name]
        return selectedFocusSteps(selection)
    }

    private func selectedFocusName(_ focus: String?) -> String {
        return focus ?? Self.emptyText
    }

    private func selectedFocusSteps(_ selection: [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)]?) -> [(label: String, action: String, section: String, mode: String, size: NSSize, check: String)] {
        return selection ?? Self.steps
    }

    private func perform(_ step: (label: String, action: String, section: String, mode: String, size: NSSize, check: String)) async throws -> Void {
        currentLabel = step.label
        currentStage = Self.actionKey
        resizeForCheck(step.size)
        try await waitForExpiry(step.action)
        let result: Any = try await evaluate(step)
        var report: [String: Any] = try reportDictionary(result)
        report[Self.nativeWindowKey] = dimensions()
        currentStage = Self.snapshotKey
        let image: NSImage = try await snapshot()
        let imageURL: URL = directory.appendingPathComponent(step.label + "-window.png")
        try imageData(image).write(to: imageURL, options: .atomic)
        report[Self.snapshotKey] = ["saved": true, "file": imageURL.lastPathComponent,
                              Self.widthKey: image.size.width, Self.heightKey: image.size.height]
        currentStage = "report"
        try writeRecord(report, directory.appendingPathComponent(step.label + "-report.json"))
    }

    private func waitForExpiry(_ action: String) async throws -> Void {
        guard action == Self.expiryAction else { return }
        try await Task.sleep(nanoseconds: Self.expiryNanoseconds)
    }

    private func resize(_ size: NSSize) -> Void {
        window.setContentSize(size)
        window.center()
        window.contentView?.layoutSubtreeIfNeeded()
    }

    private func resizeForCheck(_ size: NSSize) -> Void {
        guard ProcessInfo.processInfo.environment[Self.captureOnlyName] != Self.enabledValue else { return }
        resize(size)
    }

    private func evaluate(_ step: (label: String, action: String, section: String, mode: String, size: NSSize, check: String)) async throws -> Any {
        let input: [String: Any] = [Self.labelKey: step.label, Self.actionKey: step.action, "section": step.section,
                                  "mode": step.mode, Self.widthKey: step.size.width, Self.heightKey: step.size.height,
                                  "check": step.check, "agentRecordId": Self.environmentValue("OBSERVATORY_QA_AGENT_RECORD_ID", fallback: Self.emptyText), "actionName": Self.environmentValue("OBSERVATORY_QA_ACTION_NAME", fallback: "offline_ui_validation"),
                                  "sessionId": Self.environmentValue("OBSERVATORY_QA_SESSION_ID", fallback: Self.emptyText),
                                  "taskId": Self.environmentValue("OBSERVATORY_QA_TASK_ID", fallback: Self.emptyText),
                                  "actionRecordId": Self.environmentValue("OBSERVATORY_QA_ACTION_RECORD_ID", fallback: Self.emptyText)]
        let script: String = Self.reportScript + "\nreturn await NativePreviewReport.verify(input, iconLoaded);"
        return try await operation { (completion: @escaping @MainActor @Sendable (Result<Any, Error>) -> Void) -> Void in
            self.view.callAsyncJavaScript(script, arguments: ["input": input, Self.iconLoadedKey: NativeApplicationIcon.isLoaded()],
                                          in: nil, in: .page, completionHandler: completion)
        }
    }

    private static func environmentValue(_ name: String, fallback: String) -> String {
        return ProcessInfo.processInfo.environment[name] ?? fallback
    }

    private func snapshot() async throws -> NSImage {
        let configuration: WKSnapshotConfiguration = WKSnapshotConfiguration()
        configuration.rect = view.bounds
        return try await operation { (completion: @escaping @MainActor @Sendable (Result<NSImage, Error>) -> Void) -> Void in
            self.view.takeSnapshot(with: configuration) { (image: NSImage?, error: Error?) -> Void in
                completion(self.snapshotResult(image, error))
            }
        }
    }

    private func operation<Value>(_ start: (@escaping @MainActor @Sendable (Result<Value, Error>) -> Void) -> Void) async throws -> Value {
        operationNumber += 1
        let number: Int = operationNumber
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Value, Error>) -> Void in
            deadline(continuation, number)
            start { (result: Result<Value, Error>) -> Void in self.complete(result, continuation, number) }
        }
    }

    private func deadline<Value>(_ continuation: CheckedContinuation<Value, Error>, _ number: Int) -> Void {
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.operationSeconds) { () -> Void in
            let error: NSError = self.failure("WebKit operation exceeded its 30-second limit.")
            self.complete(.failure(error), continuation, number)
        }
    }

    private func complete<Value>(_ result: Result<Value, Error>, _ continuation: CheckedContinuation<Value, Error>, _ number: Int) -> Void {
        let current: Bool = [number == operationNumber, number > finishedOperation].allSatisfy { (value: Bool) -> Bool in value }
        completeCurrent(result, continuation, number, current)
    }

    private func completeCurrent<Value>(_ result: Result<Value, Error>, _ continuation: CheckedContinuation<Value, Error>, _ number: Int, _ current: Bool) -> Void {
        guard current else { return }
        finishedOperation = number
        continuation.resume(with: result)
    }

    private func snapshotResult(_ image: NSImage?, _ error: Error?) -> Result<NSImage, Error> {
        let failed: Result<NSImage, Error>? = snapshotError(error)
        let captured: Result<NSImage, Error> = snapshotImage(image)
        return selectedSnapshot(failed, captured)
    }

    private func snapshotError(_ error: Error?) -> Result<NSImage, Error>? {
        guard let error: Error = error else { return nil }
        return .failure(error)
    }

    private func snapshotImage(_ image: NSImage?) -> Result<NSImage, Error> {
        guard let image: NSImage = image else { return .failure(failure("WebKit returned no snapshot image.")) }
        return .success(image)
    }

    private func selectedSnapshot(_ failed: Result<NSImage, Error>?, _ captured: Result<NSImage, Error>) -> Result<NSImage, Error> {
        guard let failed: Result<NSImage, Error> = failed else { return captured }
        return failed
    }

    private func reportDictionary(_ result: Any) throws -> [String: Any] {
        let text: String = try reportText(result)
        let value: Any = try JSONSerialization.jsonObject(with: Data(text.utf8))
        return try reportObject(value)
    }

    private func reportText(_ result: Any) throws -> String {
        guard let text: String = result as? String else { throw failure("WebKit returned no report string.") }
        return text
    }

    private func reportObject(_ value: Any) throws -> [String: Any] {
        guard let report: [String: Any] = value as? [String: Any] else { throw failure("WebKit returned an invalid report object.") }
        return report
    }

    private func dimensions() -> [String: CGFloat] {
        let content: NSRect = window.contentRect(forFrameRect: window.frame)
        return ["contentWidth": content.width, "contentHeight": content.height,
                "webViewWidth": view.bounds.width, "webViewHeight": view.bounds.height]
    }

    private func imageData(_ image: NSImage) throws -> Data {
        let data: Data = try imageRepresentation(image)
        let bitmap: NSBitmapImageRep = try imageBitmap(data)
        return try imageBytes(bitmap)
    }

    private func imageRepresentation(_ image: NSImage) throws -> Data {
        guard let data: Data = image.tiffRepresentation else { throw failure("Snapshot TIFF conversion failed.") }
        return data
    }

    private func imageBitmap(_ data: Data) throws -> NSBitmapImageRep {
        guard let bitmap: NSBitmapImageRep = NSBitmapImageRep(data: data) else { throw failure("Snapshot bitmap conversion failed.") }
        return bitmap
    }

    private func imageBytes(_ bitmap: NSBitmapImageRep) throws -> Data {
        guard let bytes: Data = bitmap.representation(using: .png, properties: [:]) else { throw failure("Snapshot PNG conversion failed.") }
        return bytes
    }

    private func writeRecord(_ report: [String: Any], _ destination: URL) throws -> Void {
        let data: Data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: destination, options: .atomic)
    }

    private func failure(_ message: String) -> NSError {
        return NSError(domain: Self.errorDomain, code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }

    private func finishRun(_ error: Error?) -> Void {
        let record: [String: Any] = completionRecord(error)
        saveCompletion(record)
    }

    private func completionRecord(_ error: Error?) -> [String: Any] {
        guard let error: Error = error else { return [Self.successKey: true, Self.directoryKey: directory.path, "completedSteps": selectedSteps().count] }
        return [Self.successKey: false, Self.directoryKey: directory.path, Self.labelKey: currentLabel,
                "stage": currentStage, "error": String(error.localizedDescription.prefix(Self.maximumErrorCharacters)),
                "javascriptException": javascriptException(error as NSError),
                Self.nativeWindowKey: dimensions(), "windowVisible": window.isVisible,
                "windowKey": window.isKeyWindow, "occlusionVisible": window.occlusionState.contains(.visible)]
    }

    private func javascriptException(_ error: NSError) -> String {
        let message: String = error.userInfo["WKJavaScriptExceptionMessage"] as? String ?? Self.emptyText
        return String(message.prefix(Self.maximumErrorCharacters))
    }

    private func saveCompletion(_ record: [String: Any]) -> Void {
        do {
            try writeRecord(record, directory.deletingLastPathComponent().appendingPathComponent("native-qa-status.json"))
            print("Native QA status: " + directory.path)
        } catch {
            print("Native QA failed to write status: " + error.localizedDescription)
        }
    }

    private static func evidenceDirectory(_ root: String?) -> URL? {
        guard let root: String = root else { return nil }
        return URL(fileURLWithPath: root).appendingPathComponent("evidence")
    }

    private static func previewContext(_ view: WKWebView?, _ window: NSWindow?, _ directory: URL?) -> (WKWebView, NSWindow, URL)? {
        let entries: (WKWebView?, NSWindow?, URL?) = (view, window, directory)
        return completeContext(entries)
    }

    private static func completeContext(_ context: (WKWebView?, NSWindow?, URL?)) -> (WKWebView, NSWindow, URL)? {
        guard let view: WKWebView = context.0, let window: NSWindow = context.1,
              let directory: URL = context.2 else { return nil }
        return (view, window, directory)
    }

    private static func enabledContext(_ context: (WKWebView, NSWindow, URL)?, _ enabled: Bool) -> (WKWebView, NSWindow, URL)? {
        guard enabled else { return nil }
        return context
    }

    private static func startPreview(_ context: (WKWebView, NSWindow, URL)?) -> Void {
        guard let context: (WKWebView, NSWindow, URL) = context else { return }
        NativePreviewVerifier(context: context).start()
    }
}
