import SwiftUI

struct SearchImage: Decodable, Equatable, Identifiable {
    let image_url: String
    let thumbnail_url: String?
    let source_website_url: String?
    let caption: String?
    let image_width: Int?
    let image_height: Int?

    var id: String { image_url }

    func web_url(_ value: String?) -> URL? {
        guard let value, let url = URL(string: value),
              ["https", "http"].contains(url.scheme?.lowercased() ?? ""),
              url.host != nil else { return nil }
        return url
    }
}

struct SearchImageView: View {
    let result: SearchImage
    @State private var use_original = false

    var body: some View {
        if let original = result.web_url(result.image_url) {
            let thumbnail = result.web_url(result.thumbnail_url)
            VStack(alignment: .leading, spacing: 4) {
                AsyncImage(url: use_original ? original : thumbnail ?? original) { phase in
                    switch phase {
                    case .success(let image):
                        Link(destination: original) {
                            image.resizable().scaledToFit()
                                .accessibilityLabel(result.caption ?? "Open image")
                        }
                    case .failure:
                        if !use_original && thumbnail != nil && thumbnail != original {
                            ProgressView().onAppear { use_original = true }
                        } else {
                            Link("Open image", destination: original)
                        }
                    default:
                        ProgressView()
                    }
                }
                .frame(maxWidth: 480, maxHeight: 320, alignment: .leading)

                if let caption = result.caption, !caption.isEmpty {
                    Text(caption).font(.caption).textSelection(.enabled)
                }
                if let page = result.web_url(result.source_website_url) {
                    Link(page.host ?? page.absoluteString, destination: page).font(.caption)
                }
            }
        }
    }
}
