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
        // Two menus when they fit; at large text sizes one menu, so the pickers take one row
        // instead of two above the composer.
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 16) {
                model_menu
                effort_menu
            }
            combined_menu
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
        Picker(selection: effort_selection) {
            effort_options
        } label: {
            Image(systemName: "cellularbars")
        }
        .accessibilityLabel("Reasoning effort")
    }

    private var effort_selection: Binding<String> {
        Binding(get: { model.selected_effort }, set: { model.choose_effort($0) })
    }

    @ViewBuilder private var effort_options: some View {
        Text("Automatic").tag("")
        ForEach(effort_list, id: \.self) { effort in
            Text(effort.capitalized).tag(effort)
        }
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
                Text(effort_name)
                Image(systemName: "chevron.down").font(.caption)
            }
            .fixedSize()
        }
        .accessibilityLabel("Reasoning effort")
        .accessibilityValue(effort_name)
    }

    private var combined_menu: some View {
        Menu {
            // Inline pickers in a menu show their titles as section headers.
            model_picker.pickerStyle(.inline)
            Picker("Reasoning effort", selection: effort_selection) { effort_options }
                .pickerStyle(.inline)
        } label: {
            HStack(spacing: 6) {
                Text("\(model.selected_model) · \(effort_name)").multilineTextAlignment(.trailing)
                Image(systemName: "chevron.down").font(.caption)
            }
        }
        // Keep Model above Reasoning effort even when the menu opens upward from the composer.
        .menuOrder(.fixed)
        .accessibilityLabel("Model and reasoning effort")
        .accessibilityValue("\(model.selected_model), \(effort_name)")
    }

    private var effort_name: String {
        model.selected_effort.isEmpty ? "Automatic" : model.selected_effort.capitalized
    }
    #endif
}
