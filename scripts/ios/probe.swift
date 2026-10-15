// iOS-simulator probe: URLSession via the proxy (per-session proxy dictionary, no system settings touched).
// Run: xcrun simctl spawn <udid> ./probe <proxyPort> <upstreamPort>
import Foundation

let proxyPort = Int(CommandLine.arguments[1])!
let upPort = CommandLine.arguments[2]

final class Pinned: NSObject, URLSessionDelegate {
    // Simulates certificate pinning: accept only a cert whose issuer CN is "NetProxy Harness CA".
    func urlSession(_ s: URLSession, didReceive c: URLAuthenticationChallenge,
                    completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        guard c.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
              let trust = c.protectionSpace.serverTrust,
              let chain = SecTrustCopyCertificateChain(trust) as? [SecCertificate],
              let root = chain.last,
              (SecCertificateCopySubjectSummary(root) as String?) == "NetProxy Harness CA" else {
            completionHandler(.cancelAuthenticationChallenge, nil); return
        }
        completionHandler(.useCredential, URLCredential(trust: trust))
    }
}

func session(pinned: Bool) -> URLSession {
    let cfg = URLSessionConfiguration.ephemeral
    cfg.connectionProxyDictionary = ["HTTPSEnable": 1, "HTTPSProxy": "127.0.0.1", "HTTPSPort": proxyPort,
                                     "HTTPEnable": 1, "HTTPProxy": "127.0.0.1", "HTTPPort": proxyPort]
    cfg.timeoutIntervalForRequest = 10
    return URLSession(configuration: cfg, delegate: pinned ? Pinned() : nil, delegateQueue: nil)
}

func get(_ label: String, _ s: URLSession, _ path: String) {
    let sem = DispatchSemaphore(value: 0)
    s.dataTask(with: URL(string: "https://localhost:\(upPort)\(path)")!) { d, r, e in
        if let e = e as NSError? { print("\(label): ERROR \(e.domain) \(e.code) \(e.localizedDescription)") }
        else { print("\(label): \((r as! HTTPURLResponse).statusCode) \(d?.count ?? 0) bytes proto=\((r as? HTTPURLResponse)?.value(forHTTPHeaderField: "x-np-mock") ?? "-")") }
        sem.signal()
    }.resume()
    sem.wait()
}

get("plain-via-proxy", session(pinned: false), "/small")
get("mock-via-proxy", session(pinned: false), "/__np_mock")
get("pinned-via-proxy", session(pinned: true), "/small")
