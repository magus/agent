import AppKit
import CoreGraphics
import CryptoKit
import Darwin
import IOKit

final class desktop_control {
    private var selected_window: window_target?
    private var capture_ready = false
    private let native = native_calls.shared

    func execute(_ request: [String: Any], output_dir: URL) throws -> [String: Any] {
        var completed = false
        defer {
            if !completed {
                capture_ready = false
                selected_window = nil
            }
        }
        try require_permissions()
        let started = ProcessInfo.processInfo.systemUptime
        guard let operation = request["operation"] as? String else {
            throw control_error("Missing operation")
        }

        if operation != "capture" {
            guard capture_ready else { throw control_error("Capture the desktop before sending input") }
            // Confirm capture still works before dispatching input. Do not retain this extra frame.
            let observation = try capture(output_dir)
            if let path = observation["path"] as? String {
                try? FileManager.default.removeItem(atPath: path)
            }
        }

        var result: [String: Any] = ["operation": operation]
        switch operation {
        case "capture":
            break
        case "click", "drag", "scroll":
            let point = try read_point(request, x: "x", y: "y")
            let target = try window_at(point, selected: selected_window)
            if let expected = request["expected_window_id"] as? UInt32, target.window_id != expected {
                throw control_error("Window at coordinate changed; capture again before input")
            }
            let needs_focus = selected_window?.pid != target.pid || request["focus"] as? Bool == true
            if needs_focus && !target.preserves_selection { try native.focus(target) }
            if !target.preserves_selection || selected_window == nil { selected_window = target }
            if operation == "scroll" {
                try scroll(request, at: point, target: target)
            } else {
                try mouse(request, at: point, target: target)
            }
            result["window_id"] = target.window_id
        case "keys":
            let target = try selected_target()
            try keys(request, target: target)
        case "paste":
            let target = try selected_target()
            result["paste"] = try paste(request, target: target)
        default:
            throw control_error("Unsupported operation: \(operation)")
        }

        result["dispatch_seconds"] = ProcessInfo.processInfo.systemUptime - started
        Thread.sleep(forTimeInterval: 0.15)
        result["capture"] = try capture(output_dir)
        result["locked"] = screen_is_locked()
        capture_ready = true
        completed = true
        return result
    }

    // The local setup window calls this to show a preview while the user is present.
    func capture_for_setup(_ output_dir: URL) throws -> [String: Any] {
        var result = try capture(output_dir)
        result["locked"] = screen_is_locked()
        return result
    }

    private func selected_target() throws -> window_target {
        guard let target = selected_window else {
            throw control_error("Click a window before sending keyboard input")
        }
        let exists = window_list().contains { row in
            (row[kCGWindowNumber as String] as? UInt32) == target.window_id
                && (row[kCGWindowOwnerPID as String] as? Int32) == target.pid
        }
        guard exists else { throw control_error("Selected window no longer exists") }
        return target
    }

    private func mouse(_ request: [String: Any], at start: CGPoint, target: window_target) throws {
        let dragging = request["operation"] as? String == "drag"
        let end = dragging ? try read_point(request, x: "end_x", y: "end_y") : start
        if let value = request["waypoint_list"], !(value is [[String: Any]]) {
            throw control_error("waypoint_list must contain coordinate objects")
        }
        let waypoint_list = request["waypoint_list"] as? [[String: Any]] ?? []
        guard waypoint_list.count <= 8 else { throw control_error("Use at most eight drag waypoints") }
        var point_list = try waypoint_list.map { try read_point($0, x: "x", y: "y") }
        point_list.append(end)
        let source = CGEventSource(stateID: .hidSystemState)
        let flags = CGEventFlags(rawValue: request["flags"] as? UInt64 ?? 0)
        guard let down = CGEvent(mouseEventSource: source, mouseType: .leftMouseDown,
                                 mouseCursorPosition: start, mouseButton: .left),
              let up = CGEvent(mouseEventSource: source, mouseType: .leftMouseUp,
                               mouseCursorPosition: end, mouseButton: .left) else {
            throw control_error("Cannot create mouse events")
        }

        // AppKit tracking needs shared button state as well as routed events.
        down.flags = flags
        up.flags = flags
        down.post(tap: .cgSessionEventTap)
        defer {
            up.post(tap: .cgSessionEventTap)
            Thread.sleep(forTimeInterval: 0.05)
        }
        Thread.sleep(forTimeInterval: 0.2)
        let gesture = Int64(Date().timeIntervalSince1970.truncatingRemainder(dividingBy: 1) * 1_000_000_000)
        try native.post_mouse(.mouseMoved, point: start, target: target, source: source, gesture: gesture, flags: flags)
        try native.post_mouse(.leftMouseDown, point: start, target: target, source: source, gesture: gesture, flags: flags)

        if dragging {
            var segment_start = start
            var server_dragging = false
            for segment_end in point_list {
                for step in 1...20 {
                    let fraction = Double(step) / 20
                    let point = CGPoint(x: segment_start.x + (segment_end.x - segment_start.x) * fraction,
                                        y: segment_start.y + (segment_end.y - segment_start.y) * fraction)
                    if server_dragging {
                        Thread.sleep(forTimeInterval: 0.08)
                    } else {
                        try native.post_mouse(.leftMouseDragged, point: point, target: target, source: source, gesture: gesture, flags: flags)
                    }
                    let motion = CGEvent(mouseEventSource: source, mouseType: .leftMouseDragged,
                                         mouseCursorPosition: point, mouseButton: .left)
                    motion?.flags = flags
                    motion?.post(tap: .cgSessionEventTap)
                    Thread.sleep(forTimeInterval: 0.025)
                    // Once native window movement begins, duplicate app motion disrupts its path.
                    if !server_dragging {
                        let row = (CGWindowListCopyWindowInfo(.optionAll, 0) as? [[String: Any]])?
                            .first { $0[kCGWindowNumber as String] as? UInt32 == target.window_id }
                        if let dictionary = row?[kCGWindowBounds as String] as? [String: Any],
                           let bounds = CGRect(dictionaryRepresentation: dictionary as CFDictionary),
                           bounds.size == target.bounds.size,
                           bounds.origin != target.bounds.origin {
                            server_dragging = true
                        }
                    }
                }
                segment_start = segment_end
            }
        }
        try native.post_mouse(.leftMouseUp, point: end, target: target, source: source, gesture: gesture, flags: flags)
    }

    private func scroll(_ request: [String: Any], at point: CGPoint, target: window_target) throws {
        guard let delta = request["delta_y"] as? Int32, abs(Int64(delta)) <= 100 else {
            throw control_error("delta_y must be an integer between -100 and 100 lines")
        }
        let source = CGEventSource(stateID: .hidSystemState)
        try native.post_mouse(.mouseMoved, point: point, target: target, source: source,
                              gesture: 0)
        guard let event = CGEvent(scrollWheelEvent2Source: source, units: .line,
                                  wheelCount: 1, wheel1: delta, wheel2: 0, wheel3: 0) else {
            throw control_error("Cannot create scroll event")
        }
        try native.route(event, point: point, target: target)
    }

    private func keys(_ request: [String: Any], target: window_target) throws {
        guard let key_list = request["key_list"] as? [[String: Any]], key_list.count <= 100 else {
            throw control_error("key_list must contain at most 100 keys; use paste for text")
        }
        for key in key_list {
            guard let code = key["code"] as? UInt16 else { throw control_error("Invalid key code") }
            let flags = CGEventFlags(rawValue: key["flags"] as? UInt64 ?? 0)
            for down in [true, false] {
                guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else {
                    throw control_error("Cannot create keyboard event")
                }
                event.flags = flags
                event.postToPid(target.pid)
                Thread.sleep(forTimeInterval: 0.025)
            }
        }
    }

    private func paste(_ request: [String: Any], target: window_target) throws -> [String: Any] {
        guard let text = request["text"] as? String, text.utf8.count <= 4 * 1024 * 1024 else {
            throw control_error("text must be a string of at most 4 MiB")
        }
        let board = NSPasteboard.general
        let saved_item_list = (board.pasteboardItems ?? []).map { item in
            let copy = NSPasteboardItem()
            for type in item.types {
                if let data = item.data(forType: type) { copy.setData(data, forType: type) }
            }
            return copy
        }
        let saved_hash = clipboard_hash(saved_item_list)
        board.clearContents()
        guard board.setString(text, forType: .string) else {
            board.clearContents()
            if !saved_item_list.isEmpty { board.writeObjects(saved_item_list) }
            throw control_error("Cannot write pasteboard")
        }
        let our_change = board.changeCount
        var restored = false

        func restore() {
            guard board.changeCount == our_change else { return }
            board.clearContents()
            if !saved_item_list.isEmpty { board.writeObjects(saved_item_list) }
            restored = clipboard_hash(board.pasteboardItems ?? []) == saved_hash
        }
        defer { if !restored { restore() } }

        if request["replace"] as? Bool ?? true {
            try keys(["key_list": [["code": UInt16(0), "flags": CGEventFlags.maskCommand.rawValue]]], target: target)
        }
        try keys(["key_list": [["code": UInt16(9), "flags": CGEventFlags.maskCommand.rawValue]]], target: target)
        Thread.sleep(forTimeInterval: 1)
        restore()
        return ["utf8_bytes": text.utf8.count, "clipboard_restored": restored]
    }

    private func capture(_ output_dir: URL) throws -> [String: Any] {
        try require_permissions()
        let display = CGMainDisplayID()
        let bounds = CGDisplayBounds(display)
        let saver_level = CGWindowLevelForKey(.screenSaverWindow)
        let id_list: [UInt32] = ordered_window_list().compactMap { row in
            guard row[kCGWindowIsOnscreen as String] as? Bool == true,
                  let level = row[kCGWindowLayer as String] as? Int32,
                  level < saver_level else { return nil }
            return row[kCGWindowNumber as String] as? UInt32
        }
        // This API takes raw window IDs in a CFArray, not boxed NSNumber values.
        var pointer_list = id_list.map { UnsafeRawPointer(bitPattern: Int($0)) }
        let array = pointer_list.withUnsafeMutableBufferPointer { buffer in
            CFArrayCreate(kCFAllocatorDefault, buffer.baseAddress, buffer.count, nil)!
        }
        let image = try native.capture(bounds, window_array: array)
        let bitmap = NSBitmapImageRep(cgImage: image)
        guard let data = bitmap.representation(using: .jpeg, properties: [.compressionFactor: 0.85]) else {
            throw control_error("JPEG encoding failed")
        }
        let path = output_dir.appendingPathComponent(UUID().uuidString + ".jpg")
        try data.write(to: path, options: .atomic)
        let owned_id_set = Set(window_list().compactMap { $0[kCGWindowNumber as String] as? UInt32 })
        let menu_level = CGWindowLevelForKey(.mainMenuWindow)
        let target_list: [[String: Any]] = ordered_window_list().compactMap { row in
            guard row[kCGWindowIsOnscreen as String] as? Bool == true,
                  let layer = row[kCGWindowLayer as String] as? Int32, layer < saver_level,
                  let id = row[kCGWindowNumber as String] as? UInt32,
                  owned_id_set.contains(id) || layer == menu_level,
                  let rectangle = row[kCGWindowBounds as String] as? [String: Any] else { return nil }
            return ["window_id": id, "bounds": rectangle, "layer": layer]
        }
        return ["path": path.path, "width": image.width, "height": image.height,
                "source": "layer_ordered_windows_below_screensaver", "screen_width": bounds.width,
                "screen_height": bounds.height, "window_list": target_list]
    }
}

private struct window_target {
    let pid: Int32
    let window_id: UInt32
    let bounds: CGRect
    var event_window_id: UInt32? = nil
    var preserves_selection = false
}

private func ordered_window_list() -> [[String: Any]] {
    let rows = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []
    let sorted_list = rows.enumerated().sorted { left, right in
        let left_layer = left.element[kCGWindowLayer as String] as? Int32 ?? 0
        let right_layer = right.element[kCGWindowLayer as String] as? Int32 ?? 0
        if left_layer != right_layer { return left_layer > right_layer }
        return left.offset < right.offset
    }.map { $0.element }

    var row_by_id: [UInt32: [String: Any]] = [:]
    for row in sorted_list {
        if let id = row[kCGWindowNumber as String] as? UInt32 { row_by_id[id] = row }
    }
    var emitted_set = Set<UInt32>()
    var result: [[String: Any]] = []
    for row in sorted_list {
        guard let id = row[kCGWindowNumber as String] as? UInt32,
              !emitted_set.contains(id) else { continue }
        let associated_list = (try? native_calls.shared.associated_windows(id)) ?? []
        for member in associated_list + [id] {
            guard !emitted_set.contains(member), let member_row = row_by_id[member],
                  member_row[kCGWindowLayer as String] as? Int32
                    == row[kCGWindowLayer as String] as? Int32 else { continue }
            emitted_set.insert(member)
            result.append(member_row)
        }
    }
    return result
}

private func window_list() -> [[String: Any]] {
    let rows = ordered_window_list()
    var owned_by_pid: [Int32: Bool] = [:]
    return rows.filter { row in
        guard let pid = row[kCGWindowOwnerPID as String] as? Int32 else { return false }
        if let owned = owned_by_pid[pid] { return owned }

        var info = proc_bsdinfo()
        let count = proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, Int32(MemoryLayout<proc_bsdinfo>.size))
        let owned = count == MemoryLayout<proc_bsdinfo>.size && info.pbi_uid == getuid()
        owned_by_pid[pid] = owned
        return owned
    }
}

private func window_at(_ point: CGPoint, selected: window_target?) throws -> window_target {
    let saver_level = CGWindowLevelForKey(.screenSaverWindow)
    let menu_level = CGWindowLevelForKey(.mainMenuWindow)
    let owned_id_set = Set(window_list().compactMap { $0[kCGWindowNumber as String] as? UInt32 })
    for row in ordered_window_list() {
        guard row[kCGWindowIsOnscreen as String] as? Bool == true else { continue }
        guard let layer = row[kCGWindowLayer as String] as? Int32, layer < saver_level,
              let dictionary = row[kCGWindowBounds as String] as? [String: Any],
              let bounds = CGRect(dictionaryRepresentation: dictionary as CFDictionary), bounds.contains(point),
              let pid = row[kCGWindowOwnerPID as String] as? Int32,
              let id = row[kCGWindowNumber as String] as? UInt32 else { continue }
        guard try native_calls.shared.has_visible_pixel(point, window_id: id) else { continue }
        if layer == menu_level {
            guard let selected, owned_id_set.contains(selected.window_id) else {
                throw control_error("Click an app window before using its menu bar")
            }
            return window_target(pid: selected.pid, window_id: id, bounds: bounds,
                                event_window_id: 0, preserves_selection: true)
        }
        guard owned_id_set.contains(id) else {
            throw control_error("The visible window at this point is not owned by the current user")
        }
        let is_popup = layer >= CGWindowLevelForKey(.popUpMenuWindow)
        return window_target(pid: pid, window_id: id, bounds: bounds, preserves_selection: is_popup)
    }
    throw control_error("No app window at requested coordinate")
}

private func read_point(_ request: [String: Any], x: String, y: String) throws -> CGPoint {
    guard let px = request[x] as? Double, let py = request[y] as? Double,
          px.isFinite, py.isFinite else { throw control_error("Missing or invalid coordinates") }
    return CGPoint(x: px, y: py)
}

// Reported with each result so the model knows whether a person may be at the Mac.
private func screen_is_locked() -> Bool {
    let session = CGSessionCopyCurrentDictionary() as? [String: Any]
    if session?["CGSSessionScreenIsLocked"] as? Bool == true { return true }
    let root = IORegistryGetRootEntry(kIOMainPortDefault)
    guard root != 0 else { return false }
    defer { IOObjectRelease(root) }
    let property = IORegistryEntryCreateCFProperty(root, "IOConsoleLocked" as CFString,
                                                  kCFAllocatorDefault, 0)?.takeRetainedValue()
    return property as? Bool == true
}

private func require_permissions() throws {
    guard AXIsProcessTrusted() else {
        throw control_error("Accessibility is unavailable. Open Agent setup to check permissions.")
    }
    guard CGPreflightScreenCaptureAccess() else {
        throw control_error("Screen Recording is unavailable. Open Agent setup to check permissions.")
    }
}

private func clipboard_hash(_ item_list: [NSPasteboardItem]) -> String {
    var digest = SHA256()
    for item in item_list {
        for type in item.types.sorted(by: { $0.rawValue < $1.rawValue }) {
            digest.update(data: Data(type.rawValue.utf8))
            if let data = item.data(forType: type) { digest.update(data: data) }
        }
    }
    return digest.finalize().map { String(format: "%02x", $0) }.joined()
}

struct control_error: Error, CustomStringConvertible {
    let description: String
    init(_ description: String) { self.description = description }
}

private final class native_calls {
    static let shared = native_calls()
    private typealias main_connection_function = @convention(c) () -> Int32
    private typealias associated_windows_function = @convention(c) (Int32, UInt32) -> Unmanaged<CFArray>?
    private typealias capture_function = @convention(c) (CGRect, CFArray, UInt32) -> Unmanaged<CGImage>?
    private typealias post_function = @convention(c) (Int32, CGEvent) -> Void
    private typealias location_function = @convention(c) (CGEvent, Double, Double) -> Void
    private typealias integer_field_function = @convention(c) (CGEvent, UInt32, Int64) -> Void
    private typealias process_function = @convention(c) (Int32, UnsafeMutableRawPointer) -> Int32
    private typealias record_function = @convention(c) (UnsafeRawPointer, UnsafeRawPointer) -> Int32
    private let graphics = dlopen("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", RTLD_NOW)
    private let sky = dlopen("/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight", RTLD_NOW)
    private let carbon = dlopen("/System/Library/Frameworks/Carbon.framework/Carbon", RTLD_NOW)

    func associated_windows(_ id: UInt32) throws -> [UInt32] {
        let connection: main_connection_function = try symbol(sky, "SLSMainConnectionID")
        let copy: associated_windows_function = try symbol(sky, "SLSCopyAssociatedWindows")
        guard let value = copy(connection(), id)?.takeRetainedValue() else { return [] }
        let number_list = value as? [NSNumber] ?? []
        return number_list.map { $0.uint32Value }
    }

    func capture(_ bounds: CGRect, window_array: CFArray) throws -> CGImage {
        // The verified capture function remains exported but is unavailable in the Swift SDK.
        let make: capture_function = try symbol(graphics, "CGWindowListCreateImageFromArray")
        guard let image = make(bounds, window_array, CGWindowImageOption.bestResolution.rawValue) else {
            throw control_error("Desktop window capture unavailable")
        }
        return image.takeRetainedValue()
    }

    func has_visible_pixel(_ point: CGPoint, window_id: UInt32) throws -> Bool {
        var pointer_list = [UnsafeRawPointer(bitPattern: Int(window_id))]
        let array = pointer_list.withUnsafeMutableBufferPointer { buffer in
            CFArrayCreate(kCFAllocatorDefault, buffer.baseAddress, buffer.count, nil)!
        }
        let bounds = CGRect(x: floor(point.x), y: floor(point.y), width: 1, height: 1)
        let image = try capture(bounds, window_array: array)
        let bitmap = NSBitmapImageRep(cgImage: image)
        let color = bitmap.colorAt(x: bitmap.pixelsWide / 2, y: bitmap.pixelsHigh / 2)
        return (color?.alphaComponent ?? 0) > 0.01
    }

    func focus(_ target: window_target) throws {
        let get_process: process_function = try symbol(carbon, "GetProcessForPID")
        let post: record_function = try symbol(sky, "SLPSPostEventRecordTo")
        var process_number = (UInt32(0), UInt32(0))
        let status = withUnsafeMutablePointer(to: &process_number) { get_process(target.pid, $0) }
        guard status == 0 else { throw control_error("process_function lookup failed: \(status)") }
        for kind: UInt8 in [13, 1, 2] {
            var record = [UInt8](repeating: 0, count: 248)
            record[4] = 248
            record[8] = kind
            for index in 0..<4 { record[60 + index] = UInt8((target.window_id >> (index * 8)) & 255) }
            if kind == 13 {
                record[138] = 1
            } else {
                record[58] = 16
                for index in 32..<48 { record[index] = 255 }
            }
            let result = withUnsafePointer(to: &process_number) { pointer in
                record.withUnsafeBytes { bytes in post(pointer, bytes.baseAddress!) }
            }
            guard result == 0 else { throw control_error("Window focus failed: \(result)") }
            Thread.sleep(forTimeInterval: 0.12)
        }
    }

    func post_mouse(
        _ type: CGEventType,
        point: CGPoint,
        target: window_target,
        source: CGEventSource?,
        gesture: Int64,
        flags: CGEventFlags = []
    ) throws {
        guard let event = CGEvent(mouseEventSource: source, mouseType: type,
                                  mouseCursorPosition: point, mouseButton: .left) else {
            throw control_error("Cannot create routed mouse event")
        }
        let set: integer_field_function = try symbol(sky, "SLEventSetIntegerValueField")
        for (field, value): (UInt32, Int64) in [(0, 0), (1, 1), (3, 0), (7, 0)] {
            set(event, field, value)
        }
        set(event, 58, gesture)
        event.flags = flags
        try self.route(event, point: point, target: target)
        Thread.sleep(forTimeInterval: 0.08)
    }

    func route(
        _ event: CGEvent,
        point: CGPoint,
        target: window_target
    ) throws {
        let post: post_function = try symbol(sky, "SLEventPostToPid")
        let locate: location_function = try symbol(sky, "CGEventSetWindowLocation")
        let set: integer_field_function = try symbol(sky, "SLEventSetIntegerValueField")
        event.location = point
        // Wheel events keep this field window-local. Mouse events use desktop coordinates.
        if event.type == .scrollWheel {
            locate(event, point.x - target.bounds.minX, point.y - target.bounds.minY)
        } else {
            locate(event, point.x, point.y)
        }
        set(event, 40, Int64(target.pid))
        set(event, 51, Int64(target.event_window_id ?? target.window_id))
        for field: UInt32 in [91, 92] { set(event, field, Int64(target.window_id)) }
        post(target.pid, event)
    }

    private func symbol<T>(_ handle: UnsafeMutableRawPointer?, _ name: String) throws -> T {
        guard let handle, let pointer = dlsym(handle, name) else {
            throw control_error("Required native symbol unavailable: \(name)")
        }
        return unsafeBitCast(pointer, to: T.self)
    }
}
