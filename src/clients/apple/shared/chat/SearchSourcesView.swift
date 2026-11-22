import SwiftUI

struct SearchSourcesView: View {
    let query: String
    let service: String
    let conversation_id: String
    let entry_index: Int
    let running: Bool
    let image_list: [SearchImage]

    @State private var expanded = false
    @State private var source_list: [Source]?
    @State private var error: String?

    var body: some View {
        DisclosureGroup(isExpanded: $expanded) {
            if expanded {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(image_list) { result in
                        SearchImageView(result: result)
                    }
                    if let error {
                        Text(error)
                    } else if let source_list {
                        let groups = Dictionary(grouping: source_list, by: \.domain)
                        ForEach(source_list.filter { $0.citation != nil }) { source in
                            Link(destination: source.url) {
                                Label(source.citation ?? source.url.absoluteString, systemImage: "quote.bubble")
                            }
                            .accessibilityHint("Cited source")
                        }
                        if groups.isEmpty {
                            Text(running ? "Waiting for sources…" : "No source URLs recorded.")
                        } else {
                            DisclosureGroup("\(groups.count) sites", content: {
                                ForEach(groups.keys.sorted(), id: \.self) { domain in
                                    if let links = groups[domain] {
                                        DisclosureGroup {
                                            ForEach(links) { source in
                                                Link(source.url.absoluteString, destination: source.url)
                                            }
                                        } label: {
                                            HStack {
                                                AsyncImage(url: URL(string: "https://\(domain)/favicon.ico")) { image in
                                                    image.resizable().scaledToFit()
                                                } placeholder: {
                                                    Image(systemName: "globe").resizable().scaledToFit()
                                                }
                                                .frame(width: 16, height: 16)
                                                Text(domain)
                                            }
                                        }
                                    }
                                }
                            })
                        }
                    } else {
                        ProgressView().controlSize(.small)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, 6)
                .task(id: "\(service)/\(conversation_id)/\(entry_index)/\(running)") {
                    do {
                        error = nil
                        let result = try await Client().request([
                            "action": "search_sources", "service": service,
                            "id": conversation_id, "entry_index": entry_index,
                        ], as: [Source].self)
                        if !Task.isCancelled { source_list = result.filter(\.is_web_link) }
                    } catch {
                        if !Task.isCancelled { self.error = error.localizedDescription }
                    }
                }
            }
        } label: {
            Label(query, systemImage: "magnifyingglass")
                .accessibilityLabel("Web search: " + query)
        }
        .font(.callout)
        .foregroundStyle(.secondary)
    }
}

private struct Source: Decodable, Identifiable {
    let url: URL
    let citation: String?

    var id: URL { url }
    var domain: String { url.host?.lowercased() ?? "" }
    var is_web_link: Bool { ["https", "http"].contains(url.scheme?.lowercased() ?? "") && !domain.isEmpty }
}
