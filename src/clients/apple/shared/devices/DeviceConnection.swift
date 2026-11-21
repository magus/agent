import Foundation

struct DeviceConnection: Decodable, Identifiable, Equatable {
    let id: String
    let name: String
    let address: String
    let outgoing: Bool
    let incoming: Bool
    let pending: Bool
    let blocked: Bool
    let discovered: Bool
    let status: String
}

struct DeviceList: Decodable, Equatable {
    var name = ""
    var execution_device = false
    var device_list: [DeviceConnection] = []
}
