enum DesktopRestartDelayTests {
    static func check() {
        var retry = DesktopRestartDelay()

        for seconds in [2, 4, 8, 16, 32, 60, 60] {
            precondition(retry.next(afterRunningFor: nil) == .seconds(seconds))
        }

        precondition(retry.next(afterRunningFor: .seconds(1)) == .seconds(60))
        precondition(retry.next(afterRunningFor: .seconds(31)) == .seconds(2))
        precondition(retry.next(afterRunningFor: nil) == .seconds(4))
        print("Desktop restart delays passed")
    }
}
