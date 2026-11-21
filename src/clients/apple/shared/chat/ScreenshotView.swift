import SwiftUI

struct ScreenshotView: View {
    let path: String
    @State private var full_size = false

    var body: some View {
        if let image {
            Button { full_size = true } label: {
                image.resizable().scaledToFit().frame(maxHeight: 320)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Open screenshot")
            .sheet(isPresented: $full_size) {
                NavigationStack {
                    ScrollView([.horizontal, .vertical]) { image }
                        .toolbar {
                            ToolbarItem(placement: .confirmationAction) {
                                Button("Done") { full_size = false }
                            }
                        }
                }
                #if os(macOS)
                .frame(minWidth: 640, minHeight: 480)
                #endif
            }
        }
    }

    private var image: Image? {
        #if os(macOS)
        guard let image = NSImage(contentsOfFile: path) else { return nil }
        return Image(nsImage: image)
        #else
        guard let image = UIImage(contentsOfFile: path) else { return nil }
        return Image(uiImage: image)
        #endif
    }
}
