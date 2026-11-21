#if os(macOS)
struct DesktopPermissions {
    var accessibility = false
    var screen_recording = false
    var granted: Bool { accessibility && screen_recording }
}
#endif
