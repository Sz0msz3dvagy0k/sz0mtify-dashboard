import UIKit
import Capacitor

class AppViewController: CAPBridgeViewController {
    override open func capacitorDidLoad() {
        super.capacitorDidLoad()
        webView?.inputAssistantItem.leadingBarButtonGroups = []
        webView?.inputAssistantItem.trailingBarButtonGroups = []
    }
}
