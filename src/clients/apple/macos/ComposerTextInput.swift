#if os(macOS)
import AppKit
import SwiftUI

struct ComposerTextInput: NSViewRepresentable {
    @Binding var text: String
    @Binding var height: CGFloat
    let send: () -> Void

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll_view = NSScrollView()
        scroll_view.drawsBackground = false
        scroll_view.hasVerticalScroller = true

        let text_view = ComposerEditor()
        text_view.delegate = context.coordinator
        text_view.on_send = send
        text_view.isRichText = false
        text_view.drawsBackground = false
        text_view.font = .preferredFont(forTextStyle: .body)
        text_view.textColor = .labelColor
        text_view.insertionPointColor = .labelColor
        text_view.textContainerInset = NSSize(width: 0, height: 2)
        text_view.textContainer?.lineFragmentPadding = 0
        text_view.isVerticallyResizable = true
        text_view.isHorizontallyResizable = false
        text_view.autoresizingMask = [.width]
        text_view.textContainer?.widthTracksTextView = true
        text_view.setAccessibilityLabel("Message Agent")
        scroll_view.documentView = text_view
        return scroll_view
    }

    func updateNSView(_ scroll_view: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let text_view = scroll_view.documentView as? ComposerEditor else { return }
        text_view.on_send = send
        if text_view.string != text { text_view.string = text }
        context.coordinator.measure(text_view)
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: ComposerTextInput

        init(_ parent: ComposerTextInput) { self.parent = parent }

        func textDidChange(_ notification: Notification) {
            guard let text_view = notification.object as? NSTextView else { return }
            parent.text = text_view.string
            measure(text_view)
        }

        func measure(_ text_view: NSTextView) {
            guard let container = text_view.textContainer, let layout = text_view.layoutManager else { return }
            layout.ensureLayout(for: container)
            let height = min(180, max(46, ceil(layout.usedRect(for: container).height) + 4))
            guard parent.height != height else { return }
            DispatchQueue.main.async { [weak self] in self?.parent.height = height }
        }
    }
}

private final class ComposerEditor: NSTextView {
    var on_send: (() -> Void)?
    private var key_monitor: Any?

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if let key_monitor { NSEvent.removeMonitor(key_monitor) }
        key_monitor = nil
        guard window != nil else { return }

        key_monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, let window = self.window, window.isKeyWindow,
                  event.window === window else { return event }

            let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            if event.keyCode == 53 && modifiers == .shift {
                window.makeFirstResponder(self)
                return nil
            }
            guard !(window.firstResponder is NSTextView),
                  modifiers.intersection([.command, .control]).isEmpty,
                  let characters = event.characters,
                  !characters.isEmpty,
                  characters.unicodeScalars.allSatisfy({ !CharacterSet.controlCharacters.contains($0) && $0.value < 0xF700 }) else {
                return event
            }
            window.makeFirstResponder(self)
            self.keyDown(with: event)
            return nil
        }
    }

    // Tab moves focus, as in Messages, instead of typing a tab into the message.
    override func insertTab(_ sender: Any?) {
        window?.selectNextKeyView(sender)
    }

    override func insertBacktab(_ sender: Any?) {
        window?.selectPreviousKeyView(sender)
    }

    override func keyDown(with event: NSEvent) {
        let is_return = event.keyCode == 36 || event.keyCode == 76
        if is_return && !event.modifierFlags.contains(.shift) && !hasMarkedText() {
            on_send?()
            return
        }
        super.keyDown(with: event)
    }

    deinit {
        if let key_monitor { NSEvent.removeMonitor(key_monitor) }
    }
}
#endif
