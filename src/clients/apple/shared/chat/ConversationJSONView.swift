import SwiftUI

struct ConversationJSONTarget: Codable, Hashable {
    let service: String
    let id: String
}

struct ConversationJSONView: View {
    let service: String
    let id: String
    @Environment(\.dismiss) private var dismiss
    @State private var events = false
    @State private var decode_nested = true
    @AppStorage("conversation_json_pierre_words") private var use_words = true
    @State private var revision = 0
    @State private var document: String?
    @State private var failure: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            #if !os(macOS)
            HStack {
                Text("Conversation JSON").font(.headline)
                Spacer()
                Button("Done") { dismiss() }
            }
            #endif
            Picker("Data", selection: $events) {
                Text("Saved conversation").tag(false)
                Text("OpenAI events").tag(true)
            }
            .pickerStyle(.segmented)
            Picker("Renderer", selection: $use_words) {
                Text("Native text").tag(false)
                Text("Pierre Words").tag(true)
            }
            .pickerStyle(.segmented)
            Toggle("Decode nested JSON", isOn: $decode_nested)
            if let failure {
                Text(failure).textSelection(.enabled)
                Spacer()
            } else if let document {
                if events && document == "[]" {
                    Text("No OpenAI events were captured. Earlier turns are available under Saved conversation.")
                    Spacer()
                } else if use_words {
                    MarkdownContent(source: "```json\n\(document)\n```", scrolls: true)
                        .equatable()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    JSONTextView(source: document)
                }
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            HStack {
                Button("Refresh") { revision += 1 }
                Spacer()
                Button("Copy JSON") {
                    guard let document else { return }
                    #if os(macOS)
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(document, forType: .string)
                    #else
                    UIPasteboard.general.string = document
                    #endif
                }.disabled(document == nil)
            }
        }
        .padding()
        #if os(macOS)
        .frame(minWidth: 560, maxWidth: .infinity, minHeight: 380, maxHeight: .infinity)
        #endif
        .task(id: "\(service)/\(id)/\(events)/\(revision)/\(decode_nested)") {
            document = nil
            failure = nil
            do {
                let text = try await Client().request([
                    "action": "conversation_json", "service": service, "id": id, "events": events,
                    "decode_nested": decode_nested,
                ], as: String.self)
                if !Task.isCancelled { document = text }
            } catch {
                if !Task.isCancelled { failure = error.localizedDescription }
            }
        }
    }
}

#if os(macOS)
private struct JSONTextView: NSViewRepresentable {
    let source: String

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSTextView.scrollableTextView()
        let text = scroll.documentView as! NSTextView
        text.isEditable = false
        text.isRichText = false
        text.usesFindBar = true
        text.font = .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let text = scroll.documentView as? NSTextView, text.string != source else { return }
        text.string = source
        text.scrollRangeToVisible(NSRange(location: 0, length: 0))
    }
}
#else
private struct JSONTextView: UIViewRepresentable {
    let source: String

    func makeUIView(context: Context) -> UITextView {
        let text = UITextView()
        text.isEditable = false
        text.isFindInteractionEnabled = true
        text.font = .monospacedSystemFont(ofSize: UIFont.preferredFont(forTextStyle: .body).pointSize, weight: .regular)
        return text
    }

    func updateUIView(_ text: UITextView, context: Context) {
        guard text.text != source else { return }
        text.text = source
        text.setContentOffset(.zero, animated: false)
    }
}
#endif
