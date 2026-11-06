import CuenoraShared
import SwiftUI

@main
struct CuenoraApp: App {
    var body: some Scene {
        WindowGroup {
            Text(SharedModule.shared.name)
                .padding()
        }
    }
}
