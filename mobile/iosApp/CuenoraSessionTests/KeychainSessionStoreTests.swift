import CuenoraShared
import Security
import XCTest

final class KeychainSessionStoreTests: XCTestCase {
    private var stores: [KeychainSessionStore] = []

    override func tearDown() async throws {
        var failure: Error?
        for store in stores {
            do { try await store.clear() }
            catch { failure = error }
        }
        stores.removeAll()
        try await super.tearDown()
        if let failure { throw failure }
    }

    func testFreshInstanceReadsWholeReplacementAndClearIsIdempotent() async throws {
        let (store, namespace) = try makeStore()
        let absent = try await store.read()
        XCTAssertNil(absent)
        try await store.replace(credentials: credentials("old"))
        let fresh = try KeychainSessionStore(namespace: namespace)
        let original = try await requirePair(fresh)
        XCTAssertEqual(original.accessToken, "access-old")
        XCTAssertEqual(original.refreshToken, "refresh-old")
        try await fresh.replace(credentials: credentials("new"))
        let replaced = try await requirePair(store)
        XCTAssertEqual(replaced.accessToken, "access-new")
        XCTAssertEqual(replaced.refreshToken, "refresh-new")
        try await store.clear()
        try await store.clear()
        let cleared = try await fresh.read()
        XCTAssertNil(cleared)
    }

    func testNamespacesDoNotReadOrClearEachOther() async throws {
        let (first, _) = try makeStore()
        let (second, _) = try makeStore()
        try await first.replace(credentials: credentials("one"))
        try await second.replace(credentials: credentials("two"))
        try await first.clear()
        let remaining = try await requirePair(second)
        XCTAssertEqual(remaining.accessToken, "access-two")
        XCTAssertEqual(remaining.refreshToken, "refresh-two")
    }

    func testConcurrentInstancesNeverReturnMixedPairs() async throws {
        let (_, namespace) = try makeStore()
        let pairs = try await withThrowingTaskGroup(of: SessionCredentials.self) { group in
            for index in 0..<12 {
                group.addTask {
                    let store = try KeychainSessionStore(namespace: namespace)
                    try await store.replace(credentials: self.credentials("\(index)"))
                    return try await self.requirePair(store)
                }
            }
            var results: [SessionCredentials] = []
            for try await pair in group { results.append(pair) }
            return results
        }
        XCTAssertEqual(pairs.count, 12)
        for pair in pairs {
            XCTAssertEqual(pair.refreshToken, pair.accessToken.replacingOccurrences(of: "access-", with: "refresh-"))
        }
    }

    func testNativeItemIsDeviceOnlyAndNotSynchronized() async throws {
        let (store, namespace) = try makeStore()
        try await store.replace(credentials: credentials("metadata"))
        var query = selector(namespace)
        query[kSecReturnAttributes as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        XCTAssertEqual(SecItemCopyMatching(query as CFDictionary, &result), errSecSuccess)
        let attributes = try XCTUnwrap(result as? [String: Any])
        XCTAssertEqual(attributes[kSecAttrAccessible as String] as? String, kSecAttrAccessibleWhenUnlockedThisDeviceOnly as String)
        XCTAssertEqual(attributes[kSecAttrSynchronizable as String] as? Bool, false)
    }

    func testCorruptNativeRecordThrowsRedactedTypedError() async throws {
        let (store, namespace) = try makeStore()
        var item = selector(namespace)
        item[kSecValueData as String] = Data("{}".utf8)
        item[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        XCTAssertEqual(SecItemAdd(item as CFDictionary, nil), errSecSuccess)
        do {
            _ = try await store.read()
            XCTFail("Corrupt records must not become absence or credentials")
        } catch {
            let failure = try XCTUnwrap((error as NSError).kotlinException as? SessionStoreException)
            XCTAssertEqual(failure.reason, .corrupt)
            XCTAssertFalse(error.localizedDescription.contains("{}"))
        }
    }

    func testRejectedCredentialInputPreservesPriorRecord() async throws {
        let (store, _) = try makeStore()
        try await store.replace(credentials: credentials("retained"))
        XCTAssertThrowsError(try SessionCredentials(accessToken: "", refreshToken: "refresh-rejected"))
        let retained = try await requirePair(store)
        XCTAssertEqual(retained.accessToken, "access-retained")
        XCTAssertEqual(retained.refreshToken, "refresh-retained")
    }

    private func requirePair(_ store: KeychainSessionStore) async throws -> SessionCredentials {
        let pair = try await store.read()
        return try XCTUnwrap(pair)
    }

    private func makeStore() throws -> (KeychainSessionStore, String) {
        let namespace = "cuenora.ios-host-test.\(UUID().uuidString)"
        let store = try KeychainSessionStore(namespace: namespace)
        stores.append(store)
        return (store, namespace)
    }

    private func credentials(_ generation: String) throws -> SessionCredentials {
        try SessionCredentials(accessToken: "access-\(generation)", refreshToken: "refresh-\(generation)")
    }

    private func selector(_ namespace: String) -> [String: Any] {
        [kSecClass as String: kSecClassGenericPassword,
         kSecAttrService as String: "org.cuenora.session.v1",
         kSecAttrAccount as String: namespace,
         kSecAttrSynchronizable as String: false]
    }
}
