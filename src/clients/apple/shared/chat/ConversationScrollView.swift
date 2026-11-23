import SwiftUI

struct ConversationScrollView<Content: View>: View {
    let sent_message_id: String?
    @ViewBuilder var content: () -> Content

    @State private var position = ScrollPosition(id: "latest-message", anchor: .bottom)
    @State private var following = true
    @State private var user_scrolling = false
    @State private var content_height: CGFloat = 0

    var body: some View {
        ScrollView {
            content()
        }
        .scrollPosition($position)
        .defaultScrollAnchor(.bottom, for: .initialOffset)
        .defaultScrollAnchor(.top, for: .alignment)
        .defaultScrollAnchor(following ? .bottom : nil, for: .sizeChanges)
        .onScrollGeometryChange(for: CGFloat.self) { geometry in
            geometry.contentSize.height
        } action: { _, height in
            content_height = height
        }
        .task(id: content_height) {
            // Let asynchronous Markdown measurements finish the current layout first.
            await Task.yield()
            guard !Task.isCancelled, following, !user_scrolling else { return }
            position.scrollTo(id: "latest-message", anchor: .bottom)
        }
        .onScrollPhaseChange { _, phase, context in
            switch phase {
            case .tracking, .interacting, .decelerating:
                user_scrolling = true
                following = false
            case .idle:
                if user_scrolling {
                    let geometry = context.geometry
                    following = geometry.contentSize.height - geometry.visibleRect.maxY < 40
                }
                user_scrolling = false
            default:
                break
            }
        }
        .onChange(of: sent_message_id) { _, id in
            guard id != nil else { return }
            following = true
            position.scrollTo(id: "latest-message", anchor: .bottom)
        }
        .overlay(alignment: .bottomTrailing) {
            if !following {
                Button {
                    following = true
                    position.scrollTo(id: "latest-message", anchor: .bottom)
                } label: {
                    Image(systemName: "arrow.down")
                        .frame(width: 28, height: 28)
                }
                .buttonStyle(.bordered)
                .buttonBorderShape(.circle)
                .background(.regularMaterial, in: Circle())
                .accessibilityLabel("Latest message")
                .help("Jump to latest message")
                .padding(12)
            }
        }
    }
}
