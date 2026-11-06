import SwiftUI
import MegaNuggetCore

struct ContentView: View {
    private let device = LocalDeviceInfo().read()
    private let catalog = CustomizationCatalog()
    @AppStorage("meganugget.appearance") private var appearance = AppAppearance.system.rawValue
    @State private var diagnostics = Diagnostics()

    private var colorScheme: ColorScheme? {
        switch AppAppearance(rawValue: appearance) ?? .system {
        case .dark: return .dark
        case .light: return .light
        case .system: return nil
        }
    }

    var body: some View {
        TabView {
            NavigationView {
                List {
                    Section {
                        Image("AppLogo")
                            .resizable().interpolation(.high).scaledToFit()
                            .frame(width: 64, height: 64)
                            .accessibilityHidden(true)
                        Text("Кастомизация для iOS 15").font(.title2.bold())
                        Text("MegaNugget · Legacy. Без jailbreak. Первый этап — iPhone 7 / iOS 15.8.8.")
                            .foregroundColor(.secondary)
                        Text("Вдохновлено GoldenNugget Mobile. Системный механизм применения ещё не перенесён; ниже явно указано, что работает.")
                            .font(.footnote)
                    }
                    Section(header: Text("Устройство")) {
                        detail("Модель", device.model)
                        detail("iOS", device.systemVersion)
                        detail("Архитектура", device.architecture)
                        detail("Среда", device.isSimulator ? "Симулятор" : "Песочница приложения")
                        detail("Цель переноса", catalog.isInitialTarget(device) ? "Совпадает · не гарантия работы твиков" : "Вне первого этапа")
                    }
                    Section(header: Text("Каталог")) {
                        ForEach(CustomizationFeature.allCases) { feature in
                            NavigationLink(destination: featureDetail(feature)) {
                                VStack(alignment: .leading, spacing: 6) {
                                    Text(title(feature))
                                    Text(availability(feature)).font(.caption).foregroundColor(.secondary)
                                }.padding(.vertical, 4)
                            }
                        }
                    }
                    Section(header: Text("Перед системными изменениями")) {
                        Text("Нужны резервная копия, проверенный канал записи и план восстановления. На этом этапе MegaNugget не изменяет системные файлы, не отключает службы и не выполняет restore.")
                            .font(.footnote)
                    }
                }.navigationTitle("MegaNugget")
            }.navigationViewStyle(.stack)
                .tabItem { Label("Каталог", systemImage: "slider.horizontal.3") }

            NavigationView {
                List {
                    appearanceSection
                    Section(header: Text("Данные и подключения")) {
                        Text("Выбранная тема хранится локально. С этого экрана не выполняются сетевые запросы, pairing или резервное копирование.")
                    }
                    Section(header: Text("О проекте")) {
                        HStack(spacing: 12) {
                            Image("AppLogo").resizable().scaledToFit()
                                .frame(width: 48, height: 48).accessibilityHidden(true)
                            Text("MegaNugget").font(.headline)
                        }
                        Text("MegaNugget · AGPL-3.0. Собственный интерфейс для старых iOS; не официальный клиент GoldenNugget.")
                        Link("GoldenNugget Mobile · авторы и исходники", destination: URL(string: "https://github.com/GoldenNugget-Team/GoldenNugget-mobile")!)
                        Link("Исходники MegaNugget", destination: URL(string: "https://github.com/kik4311/MegaNugget")!)
                    }
                }.navigationTitle("Настройки")
            }.navigationViewStyle(.stack)
                .tabItem { Label("Настройки", systemImage: "gearshape") }

            NavigationView {
                List {
                    Section(header: Text("Только события приложения")) {
                        if diagnostics.entries.isEmpty { Text("Событий пока нет.") }
                        ForEach(diagnostics.entries.reversed()) { entry in
                            VStack(alignment: .leading) {
                                Text(entry.date, style: .time).foregroundColor(.secondary)
                                Text(entry.event.rawValue).font(.caption.monospaced())
                            }
                        }
                    }
                }.navigationTitle("Журнал")
            }.navigationViewStyle(.stack)
                .tabItem { Label("Журнал", systemImage: "list.bullet.rectangle") }
        }
        .preferredColorScheme(colorScheme)
        .onChange(of: appearance) { _ in diagnostics.record(.appAppearanceChanged) }
    }

    private var appearanceSection: some View {
        Section(header: Text("Оформление MegaNugget")) {
            Picker("Тема", selection: $appearance) {
                Text("Как в системе").tag(AppAppearance.system.rawValue)
                Text("Тёмная").tag(AppAppearance.dark.rawValue)
                Text("Светлая").tag(AppAppearance.light.rawValue)
            }
            Text("Меняет только это приложение — не SpringBoard, значки или системную тему.")
                .font(.footnote).foregroundColor(.secondary)
        }
    }

    @ViewBuilder private func featureDetail(_ feature: CustomizationFeature) -> some View {
        List {
            if feature == .appAppearance {
                appearanceSection
            } else {
                Section(header: Text("Доступность")) {
                    Text(availability(feature)).font(.headline)
                    Text(explanation(feature))
                    Text("Это описание направления переноса, не работающий переключатель. Изменения не применяются.")
                        .foregroundColor(.secondary)
                }
            }
        }.navigationTitle(title(feature))
    }

    private func title(_ feature: CustomizationFeature) -> String {
        switch feature {
        case .appAppearance: return "Оформление приложения"
        case .statusBar: return "Строка состояния"
        case .springBoard: return "SpringBoard"
        case .wallpapers: return "Обои"
        case .daemons: return "Системные службы"
        }
    }

    private func availability(_ feature: CustomizationFeature) -> String {
        switch catalog.availability(of: feature, for: device) {
        case .availableLocally: return "Доступно · только внутри MegaNugget"
        case .backendNotPorted: return "Недоступно · механизм iOS 15 не перенесён"
        case .outsideInitialTarget: return "Недоступно · вне первого этапа переноса"
        }
    }

    private func explanation(_ feature: CustomizationFeature) -> String {
        switch feature {
        case .appAppearance: return "Локальное оформление приложения."
        case .statusBar: return "Формат StatusBarOverrides upstream нельзя считать совместимым с iOS 15 без отдельной проверки структуры данных и восстановления."
        case .springBoard: return "Параметры и пути GoldenNugget рассчитаны на другие версии iOS. Для каждого изменения нужны проверка ключа и доступного канала записи."
        case .wallpapers: return "Механизм PosterBoard из новых iOS не переносится на iOS 15 простой заменой номера версии. Нужен отдельный сценарий для старой системы."
        case .daemons: return "Отключение служб может нарушить работу устройства. Список upstream не применяется на iOS 15 без проверки и возможности восстановления."
        }
    }

    private func detail(_ label: String, _ value: String) -> some View {
        HStack(alignment: .top) {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value).multilineTextAlignment(.trailing)
        }.font(.subheadline)
    }
}
