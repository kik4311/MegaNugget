import SwiftUI
import MegaNuggetCore

struct LegacyTweaksView: View {
    let device: DeviceInfo
    private let port = LegacyTweakPort()
    @State private var search = ""
    @State private var onlyCandidates = true
    @State private var choices: [String: LegacyTweakValue] = [:]
    @State private var preview: LegacyDraftPreview?
    @State private var showPreview = false
    @State private var error = ""

    private var entries: [LegacyTweakSummary] {
        port.catalog(for: device).filter {
            (!onlyCandidates || $0.status == .draftCandidate)
                && (search.isEmpty || ($0.title + " " + $0.id + " " + $0.section).localizedCaseInsensitiveContains(search))
        }
    }

    var body: some View {
        List {
            Section {
                Text("Код GoldenNugget Mobile · перенос на iOS 15–17").font(.headline)
                Text("Импортированы \(port.registryCount) определений реестра, группы служб и plist-компилятор. Кандидат означает только возможность создать локальный черновик, не рабочий системный твик.")
                    .font(.footnote).foregroundColor(.secondary)
                Toggle("Только кандидаты для черновика", isOn: $onlyCandidates)
            }
            Section(header: Text("Черновик · \(choices.count) настроек")) {
                Button("Скомпилировать XML локально") {
                    do {
                        preview = try port.preview(choices, for: device)
                        error = ""
                        showPreview = true
                    } catch { self.error = error.localizedDescription }
                }.disabled(choices.isEmpty)
                if !choices.isEmpty {
                    Button("Очистить черновик", role: .destructive) { choices.removeAll(); error = "" }
                }
                if !error.isEmpty { Text(error).foregroundColor(.orange).font(.footnote) }
                Text("Нет записи на устройство, restore или сетевых запросов. Черновик хранится только в памяти и исчезает при закрытии приложения.")
                    .font(.footnote).foregroundColor(.secondary)
            }
            Section(header: Text("Каталог · \(entries.count)")) {
                if entries.isEmpty {
                    Text("Кандидатов нет. Отключите фильтр, чтобы увидеть ограничения upstream и устройства.")
                        .foregroundColor(.secondary)
                }
                ForEach(entries) { entry in
                    NavigationLink(destination: LegacyTweakEditor(entry: entry, choices: $choices)) {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(entry.title)
                            Text(entry.status.explanation).font(.caption).foregroundColor(.secondary)
                            if choices[entry.id] != nil { Text("В черновике").font(.caption).foregroundColor(.purple) }
                        }.padding(.vertical, 4)
                    }
                }
            }
            Section {
                Link("GoldenNugget Mobile · исходники и авторы", destination: URL(string: "https://github.com/GoldenNugget-Team/GoldenNugget-mobile")!)
            }
        }
        .navigationTitle("Твики · черновики")
        .searchable(text: $search, prompt: "Название или ID")
        .sheet(isPresented: $showPreview) {
            NavigationView {
                List {
                    Section {
                        Text("Это не применённые настройки").font(.headline)
                        Text("XML создан локально из выбранных определений upstream. Файлы НЕ объединены с резервной копией. Нельзя восстанавливать их как полноценную замену исходным системным plist.")
                            .font(.footnote).foregroundColor(.secondary)
                    }
                    if let preview = preview {
                        ForEach(preview.files) { file in
                            Section(header: Text(file.domain + ":" + file.relativePath)) {
                                Text(file.xml).font(.caption.monospaced()).textSelection(.enabled)
                            }
                        }
                    }
                }.navigationTitle("XML-черновик")
                    .toolbar { Button("Готово") { showPreview = false } }
            }.navigationViewStyle(.stack)
        }
    }
}

private struct LegacyTweakEditor: View {
    let entry: LegacyTweakSummary
    @Binding var choices: [String: LegacyTweakValue]
    @State private var text = ""
    @State private var boolean = true
    @State private var error = ""

    var body: some View {
        List {
            Section(header: Text(entry.section)) {
                Text(entry.title).font(.headline)
                Text(entry.detail)
                Text(entry.status.explanation).font(.footnote).foregroundColor(.secondary)
                if let warning = entry.warning { Text(warning).foregroundColor(.orange) }
                Text("Диапазон upstream: \(entry.minimumIOS ?? "нижняя граница не указана") — \(entry.maximumIOS ?? "верхняя граница не указана"). Отсутствие границы не подтверждает поддержку старых iOS.")
                    .font(.footnote).foregroundColor(.secondary)
            }
            Section(header: Text("Значение черновика")) {
                if entry.isWholeDictionary {
                    Text("Фиксированный словарь upstream. Произвольное редактирование не поддерживается.")
                } else if case .bool = entry.defaultValue {
                    Toggle("Значение", isOn: $boolean).disabled(entry.status != .draftCandidate)
                } else {
                    TextField("Значение", text: $text).disabled(entry.status != .draftCandidate)
                }
                if entry.isWholeDictionary { Text("Используется словарь значений upstream, не произвольные ключи.").font(.footnote) }
                Button("Добавить / обновить черновик") { save() }.disabled(entry.status != .draftCandidate)
                if choices[entry.id] != nil {
                    Button("Убрать из черновика", role: .destructive) { choices.removeValue(forKey: entry.id) }
                }
                if !error.isEmpty { Text(error).foregroundColor(.orange).font(.footnote) }
            }
            Section(header: Text("Метаданные upstream · только чтение")) {
                Text(entry.id).font(.caption.monospaced()).textSelection(.enabled)
                Text(entry.destination).font(.caption.monospaced()).textSelection(.enabled)
                if !entry.key.isEmpty { Text(entry.key).font(.caption.monospaced()).textSelection(.enabled) }
            }
        }.navigationTitle("Твик")
            .onAppear {
                let current = choices[entry.id] ?? entry.defaultValue
                text = current.display
                if case .bool(let value) = current { boolean = value }
            }
    }

    private func save() {
        if entry.isWholeDictionary {
            choices[entry.id] = entry.defaultValue
            error = ""
            return
        }
        let value: LegacyTweakValue
        switch entry.defaultValue {
        case .bool: value = .bool(boolean)
        case .integer:
            guard let number = Int(text) else { error = "Нужно целое число."; return }
            value = .integer(number)
        case .number:
            guard let number = Double(text), number.isFinite else { error = "Нужно конечное число."; return }
            value = .number(number)
        case .text: value = .text(text)
        }
        choices[entry.id] = value
        error = ""
    }
}
