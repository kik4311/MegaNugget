import SwiftUI
import MegaNuggetCore

struct ContentView: View {
    private let device = LocalDeviceInfo().read()
    private let engine: any JailbreakEngine = UnimplementedJailbreakEngine()
    @State private var diagnostics = Diagnostics()
    @State private var status: EngineStatus?
    @State private var showRecovery = false

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    VStack(alignment: .leading, spacing: 14) {
                        Label("IOS SECURITY RESEARCH", systemImage: "shield.lefthalf.filled")
                            .font(.caption.weight(.semibold)).foregroundColor(.purple)
                        Text("Your device.\nYour research.").font(.largeTitle.bold())
                        Text("An open research platform. No hidden actions. No unverified jailbreak claims.")
                            .foregroundColor(.secondary)
                    }.padding(.vertical, 12)

                    GroupBox(label: Label("Device", systemImage: "iphone")) {
                        VStack(spacing: 12) {
                            detail("Model", device.model)
                            detail("iOS", device.systemVersion)
                            detail("Architecture", device.architecture)
                            detail("Environment", device.isSimulator ? "Simulator · not a test device" : "Application sandbox")
                            detail("Compatibility", CompatibilityManager().evaluate(device) == .researchTarget
                                   ? "Research target only" : "Unverified")
                        }.padding(.top, 12)
                    }

                    GroupBox(label: Label("Method availability", systemImage: "magnifyingglass")) {
                        VStack(alignment: .leading, spacing: 16) {
                            Text(status?.rawValue ?? "Not checked").font(.headline.monospaced())
                            Text("Installing this app does not grant kernel privileges. No verified MegaNugget jailbreak method exists yet.")
                                .font(.footnote).foregroundColor(.secondary)
                            Button {
                                diagnostics.record(.availabilityChecked)
                                status = engine.availability(for: device)
                                diagnostics.record(.engineNotImplemented)
                            } label: {
                                Label("Check jailbreak availability", systemImage: "checkmark.shield")
                                    .frame(maxWidth: .infinity).padding(.vertical, 6)
                            }.buttonStyle(.borderedProminent)
                        }.padding(.top, 12)
                    }

                    GroupBox(label: Label("Diagnostics · on device only", systemImage: "list.bullet.rectangle")) {
                        VStack(alignment: .leading, spacing: 12) {
                            if diagnostics.entries.isEmpty {
                                Text("No operations yet.").foregroundColor(.secondary)
                            }
                            ForEach(diagnostics.entries.reversed()) { entry in
                                HStack(alignment: .top) {
                                    Text(entry.date, style: .time).foregroundColor(.secondary)
                                    Text(entry.event.rawValue).font(.caption.monospaced())
                                }
                            }
                        }.frame(maxWidth: .infinity, alignment: .leading).padding(.top, 12)
                    }
                    Button {
                        diagnostics.record(.recoveryGuidanceOpened)
                        showRecovery = true
                    } label: { Label("Recovery guidance", systemImage: "lifepreserver") }
                    Text("MegaNugget 0.1.0 · AGPL-3.0\nCopyright © 2026 MegaNugget contributors. No warranty. You may redistribute and modify under AGPL-3.0; see LICENSE in the source repository.\nResearch responsibly. Only on devices you own or are authorized to test.")
                        .font(.caption).foregroundColor(.secondary)
                }.padding(22)
            }
            .navigationTitle("MegaNugget")
            .sheet(isPresented: $showRecovery) {
                NavigationView {
                    List(RecoveryManager.guidance, id: \.self) { Text($0).font(.subheadline) }
                        .navigationTitle("Recovery guidance")
                        .toolbar { Button("Done") { showRecovery = false } }
                }
            }
        }.navigationViewStyle(.stack)
    }

    private func detail(_ label: String, _ value: String) -> some View {
        HStack(alignment: .top) {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value).multilineTextAlignment(.trailing)
        }.font(.subheadline)
    }
}
