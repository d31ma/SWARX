import AppKit
import Foundation
import WebKit

@MainActor
final class NativeApplicationLifetimeVerifier {
    private static let enabledName: String = "OBSERVATORY_LIFETIME_QA"
    private static let enabledValue: String = "1"
    private static let rootName: String = "OBSERVATORY_ROOT"
    private static let modeName: String = "OBSERVATORY_LIFETIME_MODE"
    private static let helpFirst: String = "help-first"
    private static let ordinary: String = "home-help-home"
    private static let socketName: String = "OBSERVATORY_LIFETIME_SOCKET"
    private static let eventName: String = "OBSERVATORY_LIFETIME_EVENT_FILE"
    private static let helpMenu: String = "local-help"
    private static let homeMenu: String = "monitor"
    private static let pollSeconds: Double = 0.08
    private static let limitSeconds: Double = 15
    private static let eventLimitSeconds: Double = 5
    private static let maximumEventBytes: Int = 65_536
    private static let errorDomain: String = "Observatory.ApplicationLifetimeQA"
    private static let reportName: String = "native-application-lifetime-status.json"
    private static var launched: Bool = false
    private static let captureScript: String = #"""
    const sdk = globalThis.__tachyonTac;
    const reader = sdk?.service('snapshotReader');
    if (!reader || typeof reader.open !== 'function') throw new Error('Current page reader is unavailable.');
    const host = globalThis.__tachyonNativeHostCall;
    if (typeof host !== 'function') throw new Error('Native SDK transport is unavailable.');
    let ticket = null, openEnvelope = null, pageEnvelope = null;
    globalThis.__tachyonNativeHostCall = (capability, payload) => {
      if (capability === 'companion.invoke') {
        const request = JSON.parse(payload);
        if (request.service === 'snapshotReader' && request.op === 'call') {
          if (request.name === 'open') openEnvelope = payload;
          if (request.name === 'page' && request.args?.[0] === ticket?.id && request.args?.[1] === 0) pageEnvelope = payload;
        }
      }
      return host(capability, payload);
    };
    try {
      ticket = await reader.open({selected:'',query:'',pid:null});
      if (!ticket || ticket.error || ticket.pages < 1) throw new Error('Current page transfer refused.');
      let text = '';
      for (let index = 0; index < ticket.pages; index++) {
        const part = await reader.page(ticket.id, index);
        if (part.error || part.id !== ticket.id || part.index !== index) throw new Error('Current page transfer failed.');
        text += part.content;
      }
      const view = JSON.parse(text);
      const histories = (view.runtimes || []).map(group => ({id:group.id,times:(group.history || []).map(point=>point[0])}));
      const eventSeen = !expectedAgent || (view.logical || []).some(agent =>
        agent.name === expectedAgent && agent.session?.id === expectedSession && agent.task?.id === expectedTask && agent.stale === false);
      if (!pin) await reader.release(ticket.id);
      if (!openEnvelope || !pageEnvelope) throw new Error('Actual SDK request capture unavailable.');
      return JSON.stringify({samples:view.samples,elapsed:view.elapsed,histories,eventSeen,openEnvelope,pageEnvelope});
    } finally { globalThis.__tachyonNativeHostCall = host; }
    """#
    private static let refusalScript: String = #"""
    const host = globalThis.__tachyonNativeHostCall;
    if (typeof host !== 'function') throw new Error('Native SDK transport is unavailable.');
    const raw = await host('companion.invoke', envelope);
    const answer = typeof raw === 'string' ? JSON.parse(raw) : raw;
    const refused = answer && (answer.ok === false || typeof answer.error === 'string' && answer.error.length > 0);
    if (!refused) throw new Error('Retired SDK request unexpectedly succeeded.');
    return true;
    """#
    private static let readyScript: String = #"""
    return help ? document.querySelector('.local-help') !== null
      : document.querySelector('.observatory') !== null && globalThis.__tachyonTac?.service('snapshotReader') !== undefined;
    """#
    private static let continuityScript: String = #"""
    if (!(after.samples > before.samples)) throw new Error('Collector samples reset or did not advance.');
    const current = new Map(after.histories.map(group => [group.id,new Set(group.times)]));
    const retained = before.histories.some(group => group.times.some(time => current.get(group.id)?.has(time)));
    if (!retained) throw new Error('No previous measured history remains observable.');
    if (!after.eventSeen) throw new Error('Accepted Help-time report is not fresh in the current reader.');
    return true;
    """#

    private let view: WKWebView
    private let window: NSWindow
    private let root: URL
    private let environment: [String: String]
    private var operationNumber: Int = 0
    private var finishedOperation: Int = 0
    private var stage: String = "startup"
    private var records: [[String: Any]] = []

    private init(context: (WKWebView, NSWindow, URL)) {
        view = context.0
        window = context.1
        root = context.2
        environment = ProcessInfo.processInfo.environment
    }

    static func begin(view: WKWebView?, window: NSWindow?) -> Void {
        start(context(view, window, ProcessInfo.processInfo.environment))
    }

    private static func context(_ view: WKWebView?, _ window: NSWindow?, _ environment: [String: String]) -> (WKWebView, NSWindow, URL)? {
        guard environment[enabledName] == enabledValue else { return nil }
        return completeContext(view, window, environment[rootName])
    }

    private static func completeContext(_ view: WKWebView?, _ window: NSWindow?, _ path: String?) -> (WKWebView, NSWindow, URL)? {
        guard let view: WKWebView, let window: NSWindow, let path: String else {
            print("Native lifetime QA enabled without a complete owned-app context.")
            return nil
        }
        return (view, window, URL(fileURLWithPath: path))
    }

    private static func start(_ context: (WKWebView, NSWindow, URL)?) -> Void {
        context.map(launchOnce)
    }

    private static func launchOnce(_ context: (WKWebView, NSWindow, URL)) -> Void {
        guard !launched else { return }
        launched = true
        let verifier: NativeApplicationLifetimeVerifier = NativeApplicationLifetimeVerifier(context: context)
        Task { @MainActor () -> Void in await verifier.run() }
    }

    private func run() async -> Void {
        do {
            try await sequence()
            try save(success: true)
            try requestQuit()
        } catch {
            try? save(success: false, error: error)
        }
    }

    private func sequence() async throws -> Void {
        let mode: String = try selectedMode()
        let event: [String: Any] = try eventPayload()
        let initialSocket: [String: String] = try socketIdentity()
        try await firstRoute(mode, event)
        stage = "baseline-reader"
        let before: [String: Any] = try await baseline(event)
        recordObservation("baseline", before)
        stage = "help-menu"
        try menu(identifier: Self.helpMenu)
        try await waitRoute(help: true)
        try requireSocket(initialSocket)
        stage = "event-during-help"
        try await sendEvent(event)
        try requireSocket(initialSocket)
        stage = "home-menu"
        try menu(identifier: Self.homeMenu)
        try await waitRoute(help: false)
        stage = "fresh-reader"
        let after: [String: Any] = try await advancing(before, event)
        recordObservation("returned", after)
        try requireSocket(initialSocket)
        stage = "retired-reader-refusal"
        try await requireRefusal(before, key: "openEnvelope")
        stage = "retired-transfer-refusal"
        try await requireRefusal(before, key: "pageEnvelope")
        stage = "history-continuity"
        _ = try await evaluate(Self.continuityScript, arguments: ["before": safeObservation(before), "after": safeObservation(after)])
        records.append(["check": stage, "passed": true, "socketIdentity": initialSocket])
        stage = "quit-from-help"
        try menu(identifier: Self.helpMenu)
        try await waitRoute(help: true)
        try requireSocket(initialSocket)
    }

    private func selectedMode() throws -> String {
        let mode: String = environment[Self.modeName] ?? Self.ordinary
        guard [Self.ordinary, Self.helpFirst].contains(mode) else { throw failure("Unsupported lifetime QA mode.") }
        return mode
    }

    private func firstRoute(_ mode: String, _ event: [String: Any]) async throws -> Void {
        guard mode == Self.helpFirst else {
            try await waitRoute(help: false)
            return
        }
        stage = "help-first-startup"
        try await waitRoute(help: true)
        try await sendEvent(event)
        try menu(identifier: Self.homeMenu)
        try await waitRoute(help: false)
        records.append(["check": "help-first-event", "passed": true])
    }

    private func waitRoute(help: Bool) async throws -> Void {
        let started: Date = Date()
        while try await evaluate(Self.readyScript, arguments: ["help": help]) as? Bool != true {
            try await waitIteration(started, limit: Self.limitSeconds)
        }
    }

    private func capture(pin: Bool, event: [String: Any]) async throws -> [String: Any] {
        let arguments: [String: Any] = ["pin": pin, "expectedAgent": event["agent"] ?? "", "expectedSession": event["session_id"] ?? "", "expectedTask": event["task"] ?? ""]
        let answer: Any = try await evaluate(Self.captureScript, arguments: arguments)
        guard let text: String = answer as? String else { throw failure("Reader returned no QA packet.") }
        return try dictionary(Data(text.utf8))
    }

    private func baseline(_ event: [String: Any]) async throws -> [String: Any] {
        let started: Date = Date()
        var observed: [String: Any] = try await capture(pin: true, event: event)
        while !hasHistory(observed) {
            try await waitIteration(started, limit: Self.limitSeconds)
            observed = try await capture(pin: true, event: event)
        }
        return observed
    }

    private func hasHistory(_ observed: [String: Any]) -> Bool {
        let histories: [[String: Any]] = observed["histories"] as? [[String: Any]] ?? []
        return histories.contains { (group: [String: Any]) -> Bool in
            !(group["times"] as? [NSNumber] ?? []).isEmpty
        }
    }

    private func advancing(_ before: [String: Any], _ event: [String: Any]) async throws -> [String: Any] {
        let started: Date = Date()
        let samples: Int = (before["samples"] as? NSNumber)?.intValue ?? 0
        var observed: [String: Any] = try await capture(pin: false, event: event)
        while ((observed["samples"] as? NSNumber)?.intValue ?? 0) <= samples {
            try await waitIteration(started, limit: Self.limitSeconds)
            observed = try await capture(pin: false, event: event)
        }
        return observed
    }

    private func requireRefusal(_ captured: [String: Any], key: String) async throws -> Void {
        guard let envelope: String = captured[key] as? String else { throw failure("Actual page capability was not captured.") }
        _ = try await evaluate(Self.refusalScript, arguments: ["envelope": envelope])
        records.append(["check": stage, "refused": true])
    }

    private func safeObservation(_ captured: [String: Any]) -> [String: Any] {
        return ["samples": captured["samples"] ?? NSNull(), "elapsed": captured["elapsed"] ?? NSNull(),
                "histories": captured["histories"] ?? [], "eventSeen": captured["eventSeen"] ?? false]
    }

    private func recordObservation(_ name: String, _ captured: [String: Any]) -> Void {
        records.append(["check": name, "observation": safeObservation(captured)])
    }

    private func menu(identifier: String) throws -> Void {
        let items: [NSMenuItem] = menuItems()
        let item: NSMenuItem? = items.first(where: { (item: NSMenuItem) -> Bool in item.identifier?.rawValue == identifier })
        try activate(item)
    }

    private func menuItems() -> [NSMenuItem] {
        let groups: [NSMenu] = NSApplication.shared.mainMenu?.items.compactMap { (item: NSMenuItem) -> NSMenu? in item.submenu } ?? []
        return groups.flatMap { (menu: NSMenu) -> [NSMenuItem] in menu.items }
    }

    private func activate(_ item: NSMenuItem?) throws -> Void {
        guard let item: NSMenuItem, let action: Selector = item.action else { throw failure("Declared native menu action is unavailable.") }
        try requireAction(NSApplication.shared.sendAction(action, to: item.target, from: item))
    }

    private func requireAction(_ sent: Bool) throws -> Void {
        guard sent else { throw failure("Native menu action was refused.") }
    }

    private func requestQuit() throws -> Void {
        let item: NSMenuItem? = menuItems().first(where: { (item: NSMenuItem) -> Bool in item.action == #selector(NSApplication.terminate(_:)) })
        try activate(item)
    }

    private func socketPath() throws -> String {
        guard let path: String = environment[Self.socketName], path.hasPrefix("/") else { throw failure("Exact app-owned socket path is required.") }
        return path
    }

    private func socketIdentity() throws -> [String: String] {
        let attributes: [FileAttributeKey: Any] = try FileManager.default.attributesOfItem(atPath: socketPath())
        guard attributes[.type] as? FileAttributeType == .typeSocket else { throw failure("App-owned private socket is unavailable.") }
        return ["device": try identityNumber(attributes, key: .systemNumber), "inode": try identityNumber(attributes, key: .systemFileNumber), "uid": try identityNumber(attributes, key: .ownerAccountID)]
    }

    private func identityNumber(_ attributes: [FileAttributeKey: Any], key: FileAttributeKey) throws -> String {
        guard let value: NSNumber = attributes[key] as? NSNumber else { throw failure("Socket identity field is unavailable.") }
        return value.stringValue
    }

    private func requireSocket(_ expected: [String: String]) throws -> Void {
        guard try socketIdentity() == expected else { throw failure("App-owned socket identity changed during navigation.") }
    }

    private func eventPayload() throws -> [String: Any] {
        guard let path: String = environment[Self.eventName] else { throw failure("A verified own event file is required.") }
        let data: Data = try Data(contentsOf: URL(fileURLWithPath: path))
        try requireEventSize(data.count)
        let event: [String: Any] = try dictionary(data)
        try requireEventKind(event)
        return event
    }

    private func requireEventSize(_ count: Int) throws -> Void {
        guard count <= Self.maximumEventBytes else { throw failure("Own event exceeds the private IPC size bound.") }
    }

    private func requireEventKind(_ event: [String: Any]) throws -> Void {
        guard event["type"] as? String == "logical.coordination.upsert" else { throw failure("Only an actual verified own coordination upsert is supported.") }
    }

    private func dictionary(_ data: Data) throws -> [String: Any] {
        let value: Any = try JSONSerialization.jsonObject(with: data)
        guard let dictionary: [String: Any] = value as? [String: Any] else { throw failure("Expected a typed QA JSON object.") }
        return dictionary
    }

    private func sendEvent(_ event: [String: Any]) async throws -> Void {
        let process: Process = Process()
        let input: Pipe = Pipe()
        let output: Pipe = Pipe()
        let errors: Pipe = Pipe()
        process.executableURL = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/observatory-event")
        process.arguments = ["--socket", try socketPath()]
        process.standardInput = input
        process.standardOutput = output
        process.standardError = errors
        try process.run()
        let data: Data = try JSONSerialization.data(withJSONObject: event)
        input.fileHandleForWriting.write(data)
        try input.fileHandleForWriting.close()
        try await waitProcess(process)
        let receipt: [String: Any] = try dictionary(output.fileHandleForReading.readDataToEndOfFile())
        try requireReceipt(receipt, exit: process.terminationStatus)
        records.append(["check": stage, "accepted": true, "revision": receipt["revision"] ?? NSNull()])
    }

    private func waitProcess(_ process: Process) async throws -> Void {
        let started: Date = Date()
        while process.isRunning {
            try await waitProcessIteration(process, started)
        }
    }

    private func waitProcessIteration(_ process: Process, _ started: Date) async throws -> Void {
        guard Date().timeIntervalSince(started) < Self.eventLimitSeconds else {
            process.terminate()
            throw failure("Owned event helper timed out.")
        }
        try await pause()
    }

    private func requireReceipt(_ receipt: [String: Any], exit: Int32) throws -> Void {
        guard exit == 0, receipt["accepted"] as? Bool == true else { throw failure("Own Help-time event was refused.") }
    }

    private func waitIteration(_ started: Date, limit: Double) async throws -> Void {
        guard Date().timeIntervalSince(started) < limit else { throw failure("Lifetime QA route did not become ready within its bound.") }
        try await pause()
    }

    private func pause() async throws -> Void {
        try await Task.sleep(nanoseconds: UInt64(Self.pollSeconds * 1_000_000_000))
    }

    private func evaluate(_ script: String, arguments: [String: Any]) async throws -> Any {
        operationNumber += 1
        let token: Int = operationNumber
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Any, Error>) -> Void in
            view.callAsyncJavaScript(script, arguments: arguments, in: nil, in: .page) { (result: Result<Any, Error>) -> Void in
                self.complete(result, token: token, continuation: continuation)
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + Self.limitSeconds) { @MainActor () -> Void in
                self.complete(.failure(self.failure("SDK operation exceeded its bound.")), token: token, continuation: continuation)
            }
        }
    }

    private func complete(_ result: Result<Any, Error>, token: Int, continuation: CheckedContinuation<Any, Error>) -> Void {
        guard token > finishedOperation else { return }
        finishedOperation = token
        continuation.resume(with: result)
    }

    private func failure(_ message: String) -> NSError {
        return NSError(domain: Self.errorDomain, code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }

    private func safeFailure(_ error: NSError) -> String {
        guard error.domain == Self.errorDomain else { return "SDK or OS operation failed; raw error omitted." }
        return error.localizedDescription
    }

    private func save(success: Bool, error: Error? = nil) throws -> Void {
        let evidence: URL = root.appendingPathComponent("evidence")
        try FileManager.default.createDirectory(at: evidence, withIntermediateDirectories: true)
        let record: [String: Any] = ["success": success, "stage": stage, "checks": records,
                                   "mode": environment[Self.modeName] ?? Self.ordinary,
                                   "quitRequested": success, "externalExitAndSocketRemovalRequired": true,
                                   "constructorCount": "not observed", "exactCollectorStart": "not exposed",
                                   "failureCode": (error as NSError?)?.code ?? 0,
                                   "failureReason": (error as NSError?).map(safeFailure) ?? ""]
        let data: Data = try JSONSerialization.data(withJSONObject: record, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: evidence.appendingPathComponent(Self.reportName), options: .atomic)
    }
}
