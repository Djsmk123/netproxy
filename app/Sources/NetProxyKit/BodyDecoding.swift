import Foundation
import Compression

public enum BodyKind: String, Sendable { case json, text, image, form, binary, empty }

public struct DecodedBody: Sendable {
    public var kind: BodyKind
    public var text: String?           // pretty/plain text for json/text/form
    public var data: Data              // decoded bytes (what the app actually received after content-decoding)
    public var notes: [String]         // e.g. "gzip-decoded", "truncated", "brotli not decoded"
    public var contentType: String?
}

public func headerValue(_ headers: Headers, _ name: String) -> String? {
    headers.first { $0.count == 2 && $0[0].caseInsensitiveCompare(name) == .orderedSame }?[1]
}

public enum BodyDecoder {
    /// Decodes wire bytes according to the message headers. Never throws: anything undecodable is
    /// reported in `notes` and shown as the raw bytes.
    public static func decode(_ wire: Data, headers: Headers, truncated: Bool) -> DecodedBody {
        var notes: [String] = []
        if truncated { notes.append("Only the first part of this body was stored.") }
        var data = wire
        let enc = (headerValue(headers, "content-encoding") ?? "identity").lowercased().trimmingCharacters(in: .whitespaces)
        switch enc {
        case "gzip", "x-gzip":
            if let d = gunzip(wire) { data = d; notes.append("gzip decoded") }
            else { notes.append(truncated ? "gzip body is incomplete; showing raw bytes" : "gzip could not be decoded; showing raw bytes") }
        case "deflate":
            if let d = inflateZlib(wire) { data = d; notes.append("deflate decoded") }
            else { notes.append("deflate could not be decoded; showing raw bytes") }
        case "br": notes.append("Brotli-compressed; showing raw bytes (not decoded)")
        case "identity", "": break
        default: notes.append("Unsupported content-encoding \(enc)")
        }
        let ct = headerValue(headers, "content-type")
        if data.isEmpty { return DecodedBody(kind: .empty, text: nil, data: data, notes: notes, contentType: ct) }
        let mime = (ct ?? "").split(separator: ";").first.map { $0.trimmingCharacters(in: .whitespaces).lowercased() } ?? ""
        if mime.hasPrefix("image/") { return DecodedBody(kind: .image, text: nil, data: data, notes: notes, contentType: ct) }
        let looksJSON = mime.contains("json") || ((data.first == UInt8(ascii: "{") || data.first == UInt8(ascii: "[")) && mime.isEmpty)
        if looksJSON, let pretty = prettyJSON(data) {
            return DecodedBody(kind: .json, text: pretty, data: data, notes: notes, contentType: ct)
        }
        if mime == "application/x-www-form-urlencoded", let s = String(data: data, encoding: .utf8) {
            let lines = s.split(separator: "&").map { pair -> String in
                let kv = pair.split(separator: "=", maxSplits: 1).map { String($0).removingPercentEncoding ?? String($0) }
                return kv.count == 2 ? "\(kv[0]) = \(kv[1])" : kv.joined()
            }
            return DecodedBody(kind: .form, text: lines.joined(separator: "\n"), data: data, notes: notes, contentType: ct)
        }
        let textual = mime.hasPrefix("text/") || mime.contains("xml") || mime.contains("javascript") || mime.contains("html") || mime.contains("json")
        if let s = String(data: data, encoding: .utf8), textual || isMostlyPrintable(data) {
            return DecodedBody(kind: .text, text: s, data: data, notes: notes, contentType: ct)
        }
        return DecodedBody(kind: .binary, text: nil, data: data, notes: notes, contentType: ct)
    }

    public static func prettyJSON(_ data: Data) -> String? {
        guard let obj = try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed]),
              let out = try? JSONSerialization.data(withJSONObject: obj, options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes, .fragmentsAllowed]) else { return nil }
        return String(data: out, encoding: .utf8)
    }

    static func isMostlyPrintable(_ d: Data) -> Bool {
        let sample = d.prefix(2048)
        guard !sample.isEmpty else { return false }
        let bad = sample.filter { $0 < 0x09 || ($0 > 0x0D && $0 < 0x20) }.count
        return bad * 100 / sample.count < 2
    }

    /// `xxd`-style dump of the first `limit` bytes.
    public static func hexDump(_ d: Data, limit: Int = 4096) -> String {
        var out = ""
        let bytes = Array(d.prefix(limit))
        for off in stride(from: 0, to: bytes.count, by: 16) {
            let row = bytes[off..<min(off + 16, bytes.count)]
            let hex = row.map { String(format: "%02x", $0) }.joined(separator: " ")
            let asc = String(row.map { $0 >= 0x20 && $0 < 0x7F ? Character(UnicodeScalar($0)) : "." })
            out += String(format: "%08x  ", off) + hex.padding(toLength: 47, withPad: " ", startingAt: 0) + "  " + asc + "\n"
        }
        if d.count > limit { out += "… \(d.count - limit) more bytes\n" }
        return out
    }

    // MARK: compression

    public static func gunzip(_ d: Data) -> Data? {
        let b = [UInt8](d)
        guard b.count >= 18, b[0] == 0x1f, b[1] == 0x8b, b[2] == 8 else { return nil }
        let flg = b[3]
        var i = 10
        if flg & 4 != 0 { guard i + 2 <= b.count else { return nil }; i += 2 + Int(b[i]) + Int(b[i + 1]) << 8 }
        if flg & 8 != 0 { while i < b.count, b[i] != 0 { i += 1 }; i += 1 }
        if flg & 16 != 0 { while i < b.count, b[i] != 0 { i += 1 }; i += 1 }
        if flg & 2 != 0 { i += 2 }
        guard i < b.count - 8 else { return nil }
        return inflateRaw(Data(b[i..<(b.count - 8)]))
    }

    public static func inflateZlib(_ d: Data) -> Data? {
        // HTTP "deflate" is a zlib stream in practice, but some servers send raw deflate.
        if d.count > 6, d[d.startIndex] & 0x0f == 8, (Int(d[d.startIndex]) << 8 | Int(d[d.startIndex + 1])) % 31 == 0 {
            return inflateRaw(d.dropFirst(2).dropLast(4))
        }
        return inflateRaw(d)
    }

    static func inflateRaw(_ d: Data) -> Data? {
        // Apple's COMPRESSION_ZLIB is raw DEFLATE (no zlib/gzip framing).
        let src = [UInt8](d)
        var cap = max(src.count * 8, 4096)
        for _ in 0..<8 {
            var dst = [UInt8](repeating: 0, count: cap)
            let n = compression_decode_buffer(&dst, cap, src, src.count, nil, COMPRESSION_ZLIB)
            if n > 0 && n < cap { return Data(dst[0..<n]) }
            if n == 0 { return nil }
            cap *= 4
            if cap > 256 << 20 { return nil }
        }
        return nil
    }
}

public enum CurlExport {
    /// A copy-pasteable curl command for a captured request.
    public static func command(method: String, url: String, headers: Headers, body: Data?) -> String {
        func q(_ s: String) -> String { "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'" }
        let skip: Set<String> = ["host", "content-length", "connection", "proxy-connection", "accept-encoding"]
        var parts = ["curl", "-X", method, q(url)]
        for h in headers where h.count == 2 && !skip.contains(h[0].lowercased()) && !h[0].hasPrefix(":") {
            parts += ["-H", q("\(h[0]): \(h[1])")]
        }
        if let body, !body.isEmpty {
            if let s = String(data: body, encoding: .utf8) { parts += ["--data-raw", q(s)] }
            else { parts += ["--data-binary", q("<\(body.count) binary bytes>")] }
        }
        parts.append("--compressed")
        return parts.joined(separator: " ")
    }
}
