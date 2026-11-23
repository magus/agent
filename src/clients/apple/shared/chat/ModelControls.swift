import SwiftUI

struct ModelControls: View {
    @Bindable var model: ChatModel

    var body: some View {
        if !model.selected_model.isEmpty { controls }
    }

    @ViewBuilder private var controls: some View {
        #if os(macOS)
        HStack {
            Spacer()
            model_picker.labelsHidden().fixedSize()
            effort_picker.fixedSize()
        }
        #else
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 16) {
                model_menu
                effort_menu
            }
            VStack(alignment: .trailing, spacing: 8) {
                model_menu
                effort_menu
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing)
        #endif
    }

    private var model_picker: some View {
        Picker("Model", selection: Binding(
            get: { model.selected_model },
            set: { model.choose_model($0) }
        )) {
            ForEach(model.model_option_list) { option in
                Text(option.model).tag(option.model)
            }
        }
    }

    private var effort_picker: some View {
        Picker(selection: Binding(
            get: { model.selected_effort },
            set: { model.choose_effort($0) }
        )) {
            Text("Automatic").tag("")
            ForEach(effort_list, id: \.self) { effort in
                Text(effort.capitalized).tag(effort)
            }
        } label: {
            Image(systemName: "cellularbars")
        }
        .accessibilityLabel("Reasoning effort")
    }

    private var effort_list: [String] {
        model.model_option_list.first { $0.model == model.selected_model }?.reasoning_effort_list ?? []
    }

    #if os(iOS)
    private var model_menu: some View {
        Menu {
            model_picker.pickerStyle(.inline)
        } label: {
            HStack(spacing: 6) {
                Text(model.selected_model)
                Image(systemName: "chevron.down").font(.caption)
            }
            .fixedSize()
        }
        .accessibilityLabel("Model")
        .accessibilityValue(model.selected_model)
    }

    private var effort_menu: some View {
        Menu {
            effort_picker.pickerStyle(.inline)
        } label: {
            HStack(spacing: 6) {
                Image(systemName: "cellularbars")
                Text(model.selected_effort.isEmpty ? "Automatic" : model.selected_effort.capitalized)
                Image(systemName: "chevron.down").font(.caption)
            }
            .fixedSize()
        }
        .accessibilityLabel("Reasoning effort")
        .accessibilityValue(model.selected_effort.isEmpty ? "Automatic" : model.selected_effort)
    }
    #endif
}
