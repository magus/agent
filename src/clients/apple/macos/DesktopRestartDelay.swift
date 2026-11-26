struct DesktopRestartDelay {
    private var delay: Duration = .seconds(2)

    mutating func next(afterRunningFor runtime: Duration?) -> Duration {
        if let runtime, runtime > .seconds(30) {
            delay = .seconds(2)
        }

        let current = delay
        delay = min(delay * 2, .seconds(60))
        return current
    }
}
