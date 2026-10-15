import XCTest
@testable import NetProxyKit

final class DecodingTests: XCTestCase {
    func env(_ json: String) throws -> Envelope { try Wire.decoder.decode(Envelope.self, from: Data(json.utf8)) }

    func testEveryEventKindDecodesFromRustWireFormat() throws {
        // Lines copied from the Rust serializer (np-model): flattened envelope, snake_case.
        let samples: [(String, (Event) -> Bool)] = [
            (#"{"run":"r","seq":1,"ts_us":5,"kind":"engine_started","listen":"127.0.0.1:1","engine":"hudsucker","engine_version":"0.25"}"#, { if case .engineStarted(let l) = $0 { return l == "127.0.0.1:1" } else { return false } }),
            (#"{"run":"r","seq":2,"ts_us":5,"kind":"connection_opened","conn":7,"client":"127.0.0.1:5000","target":"a.com:443"}"#, { if case .connectionOpened(7, _, "a.com:443") = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":3,"ts_us":5,"kind":"tls_hello","conn":7,"sni":"a.com","alpn":["h2"]}"#, { if case .tlsHello(7, "a.com", let a) = $0 { return a == ["h2"] } else { return false } }),
            (#"{"run":"r","seq":4,"ts_us":5,"kind":"client_tls_failed","conn":7,"class":"rejected_certificate","detail":"x"}"#, { if case .clientTlsFailed(7, .rejectedCertificate, _) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":5,"ts_us":5,"kind":"request_head","flow":9,"conn":7,"client":"c","method":"GET","uri":"https://a.com/p","host":"a.com","http_version":"HTTP/2.0","headers":[["accept","*/*"]]}"#, { if case .requestHead(let r) = $0 { return r.flow == 9 && r.headers == [["accept", "*/*"]] && r.httpVersion == "HTTP/2.0" } else { return false } }),
            (#"{"run":"r","seq":6,"ts_us":5,"kind":"response_head","flow":9,"status":200,"ttfb_us":12,"headers":[],"mocked":true}"#, { if case .responseHead(9, 200, 12, _, true) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":7,"ts_us":5,"kind":"body_end","flow":9,"dir":"res","total":18,"chunks":1,"sha256":"ab","preview_b64":"e30=","stored_bytes":18,"truncated":false}"#, { if case .bodyEnd(let b) = $0 { return b.total == 18 && b.dir == .res && b.previewB64 == "e30=" && !b.truncated } else { return false } }),
            (#"{"run":"r","seq":8,"ts_us":5,"kind":"flow_completed","flow":9,"status":200,"mocked":false}"#, { if case .flowCompleted(9, 200, false) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":9,"ts_us":5,"kind":"flow_failed","flow":9,"class":"tls_expired","detail":"d"}"#, { if case .flowFailed(9, "tls_expired", _) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":10,"ts_us":5,"kind":"events_dropped","count":42}"#, { if case .eventsDropped(42) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":11,"ts_us":5,"kind":"connection_closed","conn":7,"requests":0}"#, { if case .connectionClosed(7, 0) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":12,"ts_us":5,"kind":"body_first_byte","flow":9,"dir":"res"}"#, { if case .progress(9) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":13,"ts_us":5,"kind":"ws_message","client":"c","dir":"client_to_server","bytes":5}"#, { if case .wsMessage(_, "client_to_server", 5) = $0 { return true } else { return false } }),
            (#"{"run":"r","seq":14,"ts_us":5,"kind":"something_new","x":1}"#, { if case .unknown("something_new") = $0 { return true } else { return false } }),
        ]
        for (json, check) in samples {
            let e = try env(json)
            XCTAssertTrue(check(e.event), "wrong decode for \(e.kind): \(e.event)")
        }
    }

    func testFlowRowDecodesAndComputesDuration() throws {
        let j = #"{"run":"r","flow":3,"conn":1,"state":"completed","client":"c","method":"GET","uri":"https://a.com/x","host":"a.com","path":"/x","http_version":"HTTP/1.1","started_us":1000,"ended_us":4500,"status":200,"ttfb_us":300,"req_bytes":0,"res_bytes":18,"req_sha256":null,"res_sha256":"ab","mocked":false,"rule":null,"fail_class":null,"fail_detail":null,"first_seq":2,"last_seq":6}"#
        let r = try Wire.decoder.decode(FlowRow.self, from: Data(j.utf8))
        XCTAssertEqual(r.durationUs, 3500)
        XCTAssertEqual(r.id, "r#3")
        XCTAssertFalse(r.isSetupPage)
    }
}

final class BodyDecoderTests: XCTestCase {
    func gzip(_ s: String) throws -> Data {
        let p = Process(); p.executableURL = URL(fileURLWithPath: "/usr/bin/gzip"); p.arguments = ["-c", "-n"]
        let i = Pipe(), o = Pipe(); p.standardInput = i; p.standardOutput = o
        try p.run(); i.fileHandleForWriting.write(Data(s.utf8)); try i.fileHandleForWriting.close()
        let d = o.fileHandleForReading.readDataToEndOfFile(); p.waitUntilExit(); return d
    }

    func testGzipJSONIsDecodedAndPrettyPrinted() throws {
        let body = try gzip(#"{"b":1,"a":[true,null]}"#)
        let d = BodyDecoder.decode(body, headers: [["content-type", "application/json"], ["content-encoding", "gzip"]], truncated: false)
        XCTAssertEqual(d.kind, .json)
        XCTAssertTrue(d.notes.contains("gzip decoded"))
        XCTAssertTrue(d.text!.contains("\"a\" : ["), d.text!)          // sorted keys, pretty
        XCTAssertLessThan(d.text!.range(of: "\"a\"")!.lowerBound, d.text!.range(of: "\"b\"")!.lowerBound)
    }

    func testBigGzipRoundTrip() throws {
        let text = String(repeating: "hello netproxy ", count: 20_000)
        let d = BodyDecoder.decode(try gzip(text), headers: [["content-type", "text/plain"], ["content-encoding", "gzip"]], truncated: false)
        XCTAssertEqual(d.text, text)
    }

    func testTruncatedGzipDegradesGracefully() throws {
        let full = try gzip(String(repeating: "x", count: 100_000))
        let d = BodyDecoder.decode(full.prefix(full.count / 2), headers: [["content-encoding", "gzip"], ["content-type", "text/plain"]], truncated: true)
        XCTAssertTrue(d.notes.contains { $0.contains("incomplete") }, "\(d.notes)")
        XCTAssertTrue(d.notes.contains { $0.contains("first part") })
    }

    func testKindsAndHex() {
        XCTAssertEqual(BodyDecoder.decode(Data(), headers: [], truncated: false).kind, .empty)
        XCTAssertEqual(BodyDecoder.decode(Data([0xff, 0xd8, 0x00, 0x01]), headers: [["content-type", "image/jpeg"]], truncated: false).kind, .image)
        XCTAssertEqual(BodyDecoder.decode(Data([0, 1, 2, 3, 0xfe]), headers: [], truncated: false).kind, .binary)
        XCTAssertEqual(BodyDecoder.decode(Data("a=1&b=x%20y".utf8), headers: [["content-type", "application/x-www-form-urlencoded"]], truncated: false).text, "a = 1\nb = x y")
        XCTAssertEqual(BodyDecoder.decode(Data("br".utf8), headers: [["content-encoding", "br"]], truncated: false).notes.first?.contains("Brotli"), true)
        let hex = BodyDecoder.hexDump(Data("Hello, hex dump!!".utf8))
        XCTAssertTrue(hex.hasPrefix("00000000  48 65 6c 6c 6f 2c 20 68 65 78 20 64 75 6d 70 21"), hex)
        XCTAssertTrue(hex.contains("Hello, hex dump!"))
    }

    func testCurlExportEscapesAndFiltersHeaders() {
        let c = CurlExport.command(method: "POST", url: "https://a.com/x?y=1", headers: [["Host", "a.com"], ["Content-Type", "application/json"], ["X-Q", "it's"]], body: Data(#"{"k":"it's"}"#.utf8))
        XCTAssertTrue(c.hasPrefix("curl -X POST 'https://a.com/x?y=1'"))
        XCTAssertFalse(c.contains("Host:"))
        XCTAssertTrue(c.contains(#"-H 'X-Q: it'\''s'"#), c)
        XCTAssertTrue(c.contains(#"--data-raw '{"k":"it'\''s"}'"#), c)
    }
}

final class SupportTests: XCTestCase {
    func testFormatting() {
        XCTAssertEqual(Format.bytes(512), "512 B")
        XCTAssertEqual(Format.bytes(1536), "1.5 KB")
        XCTAssertEqual(Format.bytes(5 * 1024 * 1024), "5.0 MB")
        XCTAssertEqual(Format.duration(us: 850), "850 µs")
        XCTAssertEqual(Format.duration(us: 2_500), "2.5 ms")
        XCTAssertEqual(Format.duration(us: 1_250_000), "1.25 s")
        XCTAssertEqual(Format.duration(us: nil), "—")
    }

    func testEveryKnownFailureClassHasAnExplanation() {
        for c in ["tls_untrusted_issuer", "tls_hostname_mismatch", "tls_expired", "tls_not_yet_valid", "tls_certificate_other", "tls_handshake_failed", "connect_refused", "connect_timeout", "dns_failure", "other"] {
            XCTAssertNotNil(Explain.failure(c), c)
        }
        XCTAssertNil(Explain.failure(nil))
        XCTAssertTrue(Explain.clientTLS(.rejectedCertificate).hint.contains("Android"))
        XCTAssertTrue(Explain.clientTLS(.closedDuringHandshake).hint.localizedCaseInsensitiveContains("pinning"))
    }

    func testFingerprintMatchesOpenSSL() throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("fp-\(UUID().uuidString.prefix(6))")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tmp) }
        let key = tmp.appendingPathComponent("k.pem"), cert = tmp.appendingPathComponent("c.pem")
        let gen = Process(); gen.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
        gen.arguments = ["req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-nodes", "-keyout", key.path, "-out", cert.path, "-subj", "/CN=fp-test", "-days", "2"]
        gen.standardError = FileHandle.nullDevice; try gen.run(); gen.waitUntilExit()
        let fp = Process(); fp.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
        fp.arguments = ["x509", "-in", cert.path, "-noout", "-fingerprint", "-sha256"]
        let o = Pipe(); fp.standardOutput = o; try fp.run(); fp.waitUntilExit()
        let ref = String(decoding: o.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).split(separator: "=").last.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
        XCTAssertEqual(CertInfo.sha256Fingerprint(pem: try String(contentsOf: cert, encoding: .utf8)), ref)
    }

    func testLanAddressesAreRoutableIPv4() {
        for a in NetworkInfo.lanAddresses() {
            XCTAssertFalse(a.ip.hasPrefix("127."))
            XCTAssertEqual(a.ip.split(separator: ".").count, 4)
        }
    }
}
