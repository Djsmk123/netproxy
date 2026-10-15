import Foundation
import CryptoKit

/// Plain-language explanations shown next to failures; kept here so they are unit-tested.
public enum Explain {
    public static func failure(_ cls: String?) -> (title: String, hint: String)? {
        switch cls {
        case "tls_untrusted_issuer": return ("Server certificate is not trusted", "The server's certificate isn't signed by an authority NetProxy trusts. For a test server, add its CA to the engine's upstream trust (--upstream-ca).")
        case "tls_hostname_mismatch": return ("Certificate is for a different host", "The server presented a certificate that doesn't match the requested host name.")
        case "tls_expired": return ("Server certificate has expired", "Renew the certificate on the server, or check this Mac's clock.")
        case "tls_not_yet_valid": return ("Server certificate is not valid yet", "Check this Mac's clock, or the server's certificate dates.")
        case "tls_certificate_other", "tls_handshake_failed": return ("TLS handshake with the server failed", "The server refused or aborted the secure connection.")
        case "connect_refused": return ("Connection refused", "Nothing is listening at that address and port.")
        case "connect_timeout": return ("Connection timed out", "The server didn't answer in time. Is it reachable from this Mac?")
        case "dns_failure": return ("Host name could not be resolved", "DNS lookup failed for the requested host.")
        case .some: return ("Request failed", "The connection to the server failed.")
        case .none: return nil
        }
    }

    public static func clientTLS(_ cls: ClientTlsClass) -> (title: String, hint: String) {
        switch cls {
        case .rejectedUnknownCa: return ("Client doesn't trust the NetProxy certificate", "Install and trust the NetProxy root certificate on that device (Device Setup).")
        case .rejectedCertificate: return ("Client rejected the NetProxy certificate", "Typical of Android apps: since Android 7, apps ignore user-installed certificates unless they opt in (a debug-only network security config). Browsers and debug builds can be inspected; most release apps cannot.")
        case .closedDuringHandshake: return ("Client closed the connection during the TLS handshake", "Often certificate pinning: the app only accepts its own server certificate, so its HTTPS traffic can't be decrypted. It can also be a client that simply gave up.")
        case .other: return ("Client TLS handshake failed", "The device and NetProxy couldn't complete a secure connection.")
        }
    }
}

public enum Format {
    public static func bytes(_ n: Int64) -> String {
        if n < 1024 { return "\(n) B" }
        let units = ["KB", "MB", "GB"]
        var v = Double(n) / 1024, i = 0
        while v >= 1024 && i < units.count - 1 { v /= 1024; i += 1 }
        return String(format: v < 10 ? "%.1f %@" : "%.0f %@", v, units[i])
    }
    public static func duration(us: Int64?) -> String {
        guard let us else { return "—" }
        if us < 1_000 { return "\(us) µs" }
        if us < 1_000_000 { return String(format: us < 10_000 ? "%.1f ms" : "%.0f ms", Double(us) / 1_000) }
        return String(format: "%.2f s", Double(us) / 1_000_000)
    }
    public static func time(us: Int64) -> String {
        let d = Date(timeIntervalSince1970: Double(us) / 1_000_000)
        return d.formatted(.dateTime.hour().minute().second())
    }
}

public enum NetworkInfo {
    public struct Address: Identifiable, Equatable, Sendable { public var name: String; public var ip: String; public var id: String { name + ip } }

    /// Non-loopback IPv4 addresses of interfaces that are up: where a phone on the same network can reach this Mac.
    public static func lanAddresses() -> [Address] {
        var out: [Address] = []
        var ifap: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifap) == 0, let first = ifap else { return [] }
        defer { freeifaddrs(ifap) }
        for p in sequence(first: first, next: { $0.pointee.ifa_next }) {
            let f = p.pointee
            guard let sa = f.ifa_addr, sa.pointee.sa_family == UInt8(AF_INET),
                  Int32(f.ifa_flags) & IFF_UP != 0, Int32(f.ifa_flags) & IFF_LOOPBACK == 0 else { continue }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            if getnameinfo(sa, socklen_t(sa.pointee.sa_len), &host, socklen_t(host.count), nil, 0, NI_NUMERICHOST) == 0 {
                let ip = String(cString: host)
                if !ip.hasPrefix("169.254.") { out.append(Address(name: String(cString: f.ifa_name), ip: ip)) }
            }
        }
        return out
    }
}

public enum CertInfo {
    /// DER bytes from a PEM certificate.
    public static func der(fromPEM pem: String) -> Data? {
        let body = pem.split(separator: "\n").filter { !$0.hasPrefix("-----") }.joined()
        return Data(base64Encoded: body)
    }
    /// "AB:CD:…" SHA-256 fingerprint, as shown by Keychain Access and device certificate pages.
    public static func sha256Fingerprint(pem: String) -> String? {
        guard let d = der(fromPEM: pem) else { return nil }
        return SHA256.hash(data: d).map { String(format: "%02X", $0) }.joined(separator: ":")
    }
}
