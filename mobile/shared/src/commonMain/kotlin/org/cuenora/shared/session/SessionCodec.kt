package org.cuenora.shared.session

import kotlinx.serialization.json.*

internal object SessionCodec {
    fun encode(credentials: SessionCredentials): ByteArray = buildJsonObject {
        put("version", 1)
        put("access", credentials.accessToken)
        put("refresh", credentials.refreshToken)
    }.toString().encodeToByteArray()

    fun decode(bytes: ByteArray): SessionCredentials {
        try {
            require(bytes.size in 1..MAX_SESSION_BYTES)
            val record = Json.parseToJsonElement(bytes.decodeToString(throwOnInvalidSequence = true)) as JsonObject
            require(record.keys == setOf("version", "access", "refresh") && record["version"] == JsonPrimitive(1))
            fun secret(name: String): String {
                val value = record[name] as JsonPrimitive
                require(value.isString)
                return value.content
            }
            return SessionCredentials(secret("access"), secret("refresh"))
        } catch (_: Exception) {
            throw SessionStoreException(SessionFailure.Corrupt)
        }
    }
}
