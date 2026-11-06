import Foundation
import Darwin

// One request per connection. The socket is private to the logged-in user.
enum local_socket {
    static func serve(_ path: String, handle: (Data) -> Data) throws {
        let fd = socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else { throw control_error("Cannot create local socket") }
        defer { close(fd) }
        var address = try make_address(path)

        if FileManager.default.fileExists(atPath: path) {
            let probe = socket(AF_UNIX, SOCK_STREAM, 0)
            let connected = with_address(&address) { connect(probe, $0, $1) == 0 }
            close(probe)
            guard !connected else { throw control_error("A local agent is already running") }
            var info = stat()
            guard lstat(path, &info) == 0, info.st_uid == getuid(),
                  (info.st_mode & S_IFMT) == S_IFSOCK else {
                throw control_error("Refusing to remove an unexpected socket path")
            }
            guard unlink(path) == 0 else { throw control_error("Cannot remove stale socket") }
        }

        guard with_address(&address, { bind(fd, $0, $1) }) == 0 else {
            throw control_error("Cannot bind local socket")
        }
        defer { unlink(path) }
        guard chmod(path, 0o600) == 0, listen(fd, 8) == 0 else {
            throw control_error("Cannot listen on private socket")
        }
        signal(SIGPIPE, SIG_IGN)

        while true {
            let client = accept(fd, nil, nil)
            if client < 0 {
                if errno == EINTR { continue }
                throw control_error("Local accept failed")
            }
            configure_timeout(client)
            do {
                let request = try read_line(client)
                let response = handle(request)
                try write_all(client, response + Data([10]))
            } catch {
                let error_data = try JSONSerialization.data(withJSONObject: [
                    "ok": false, "error": String(describing: error), "automatic_retry": false,
                ])
                try? write_all(client, error_data + Data([10]))
            }
            close(client)
        }
    }

    static func send(_ path: String, request: Data) throws -> Data {
        let fd = socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else { throw control_error("Cannot create local client") }
        defer { close(fd) }
        configure_timeout(fd)
        var address = try make_address(path)
        guard with_address(&address, { connect(fd, $0, $1) }) == 0 else {
            throw control_error("Cannot connect to the local agent")
        }
        signal(SIGPIPE, SIG_IGN)
        // Normalize formatting so a multiline JSON file remains one wire record.
        let object = try JSONSerialization.jsonObject(with: request)
        let data = try JSONSerialization.data(withJSONObject: object)
        try write_all(fd, data + Data([10]))
        return try read_line(fd)
    }

    private static func configure_timeout(_ fd: Int32) {
        var timeout = timeval(tv_sec: 30, tv_usec: 0)
        setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, socklen_t(MemoryLayout<timeval>.size))
        setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &timeout, socklen_t(MemoryLayout<timeval>.size))
    }

    private static func read_line(_ fd: Int32) throws -> Data {
        var result = Data()
        var buffer = [UInt8](repeating: 0, count: 8192)
        while result.count <= 8 * 1024 * 1024 {
            let count = read(fd, &buffer, buffer.count)
            guard count > 0 else { throw control_error("Connection ended before a complete response") }
            if let newline = buffer[..<count].firstIndex(of: 10) {
                result.append(contentsOf: buffer[..<newline])
                return result
            }
            result.append(contentsOf: buffer[..<count])
        }
        throw control_error("Local request exceeds 8 MiB")
    }

    private static func write_all(_ fd: Int32, _ data: Data) throws {
        try data.withUnsafeBytes { bytes in
            var offset = 0
            while offset < bytes.count {
                let count = write(fd, bytes.baseAddress!.advanced(by: offset), bytes.count - offset)
                guard count > 0 else { throw control_error("Local response write failed") }
                offset += count
            }
        }
    }

    private static func make_address(_ path: String) throws -> sockaddr_un {
        var address = sockaddr_un()
        let data = Array(path.utf8) + [0]
        guard data.count <= MemoryLayout.size(ofValue: address.sun_path) else {
            throw control_error("Local socket path is too long")
        }
        address.sun_family = sa_family_t(AF_UNIX)
        address.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
        withUnsafeMutableBytes(of: &address.sun_path) { bytes in
            bytes.copyBytes(from: data)
        }
        return address
    }

    private static func with_address<T>(
        _ address: inout sockaddr_un,
        _ body: (UnsafePointer<sockaddr>, socklen_t) -> T
    ) -> T {
        withUnsafePointer(to: &address) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { socket_pointer in
                body(socket_pointer, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }
    }
}
