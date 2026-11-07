package org.cuenora.shared.validation

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class AuthValidationTest {
    @Test
    fun registrationUsesMailboxAndSignupPasswordPolicy() {
        assertTrue(validateRegistrationForm("person@example.test", "long enough password").isLocallyValid)
        assertTrue(validateRegistrationForm("person@例え.テスト", "long enough password").isLocallyValid)
        assertTrue(validateRegistrationForm("person@example.test", "123456789012345").isLocallyValid)
        assertIssue(validateRegistrationForm("person@example.test", "short"), "password", "invalid_password_length")
        assertIssue(validateRegistrationForm(".person@example.test", "long enough password"), "email", "invalid_email")
        assertIssue(validateRegistrationForm("person..name@example.test", "long enough password"), "email", "invalid_email")
        assertIssue(validateRegistrationForm("person@example.test", "x".repeat(129)), "password", "invalid_password_length")
    }

    @Test
    fun loginDoesNotApplySignupMinimumOrCommonPasswordPolicy() {
        assertTrue(validateLoginForm("person@example.test", "a").isLocallyValid)
        assertTrue(validateLoginForm("person@example.test", "password").isLocallyValid)
        assertTrue(validateLoginForm("person@例え.テスト", "  exact password  ").isLocallyValid)
        assertIssue(validateLoginForm("not-an-email", "a"), "email", "invalid_email")
        assertIssue(validateLoginForm("person@example.test", "x".repeat(129)), "password", "invalid_password_length")
        assertIssue(validateLoginForm("person@example.test", "é".repeat(257)), "password", "password_too_long")
    }

    @Test
    fun loginEnforcesBackendByteBoundaryWithoutTrimmingPasswords() {
        assertTrue(validateLoginForm("person@example.test", "😀".repeat(128)).isLocallyValid)
        assertIssue(validateLoginForm("person@example.test", "😀".repeat(129)), "password", "password_too_long")
        assertTrue(validateLoginForm("person@example.test", " password ").isLocallyValid)
    }

    @Test
    fun recoveryAndResetValidateEmailTokenAndNewPasswordIndependently() {
        assertTrue(validateRecoveryEmailForm("person@example.test").isLocallyValid)
        assertIssue(validateRecoveryEmailForm("person@@example.test"), "email", "invalid_email")

        val recoveryToken = "pr1_" + "a".repeat(64)
        val verificationToken = "ev1_" + "f".repeat(64)
        assertTrue(validatePasswordResetForm(recoveryToken, "new exact password").isLocallyValid)
        assertTrue(validateEmailVerificationToken(verificationToken).isLocallyValid)
        assertIssue(validatePasswordResetForm("ev1_" + "a".repeat(64), "new exact password"), "token", "invalid_token")
        assertIssue(validatePasswordResetForm(recoveryToken, "tiny"), "password", "invalid_password_length")
        assertIssue(validateEmailVerificationToken(recoveryToken), "token", "invalid_token")
    }

    @Test
    fun tokensRequireExactPurposePrefixLengthAndLowercaseHex() {
        val validRecovery = "pr1_" + "0123456789abcdef".repeat(4)
        val validVerification = "ev1_" + "abcdef0123456789".repeat(4)
        assertTrue(validatePasswordResetForm(validRecovery, "valid password at least").isLocallyValid)
        assertTrue(validateEmailVerificationToken(validVerification).isLocallyValid)
        for (invalid in listOf(
            "pr1_" + "a".repeat(63),
            "pr1_" + "a".repeat(65),
            "PR1_" + "a".repeat(64),
            "pr1_" + "A".repeat(64),
            "pr1_" + "g" + "a".repeat(63),
            "ev1_" + "a".repeat(64),
        )) {
            assertIssue(validatePasswordResetForm(invalid, "valid password at least"), "token", "invalid_token")
        }
        assertIssue(validateEmailVerificationToken("ev1_" + "a".repeat(63)), "token", "invalid_token")
        assertIssue(validateEmailVerificationToken("PR1_" + "a".repeat(64)), "token", "invalid_token")
    }

    @Test
    fun emailChecksBoundMailboxStructureAndInputLength() {
        assertTrue(validateRecoveryEmailForm("a@example.test").isLocallyValid)
        assertTrue(validateRecoveryEmailForm("user@例え。テスト").isLocallyValid)
        assertTrue(validateRecoveryEmailForm("user@example．com").isLocallyValid)
        assertTrue(validateRecoveryEmailForm("user@example｡com").isLocallyValid)
        assertTrue(validateRecoveryEmailForm("a".repeat(64) + "@example.test").isLocallyValid)
        assertIssue(validateRecoveryEmailForm("a".repeat(65) + "@example.test"), "email", "invalid_email")
        assertIssue(validateRecoveryEmailForm("a@example"), "email", "invalid_email")
        assertIssue(validateRecoveryEmailForm("a@example..test"), "email", "invalid_email")
        assertIssue(validateRecoveryEmailForm("a@-example.test"), "email", "invalid_email")
        assertIssue(validateRecoveryEmailForm("a@example.test-"), "email", "invalid_email")
        assertIssue(validateRecoveryEmailForm("a b@example.test"), "email", "invalid_email")
        assertIssue(validateRecoveryEmailForm("a\n@example.test"), "email", "invalid_email")
        assertIssue(validateRecoveryEmailForm("a@example.test" + "x".repeat(1024)), "email", "invalid_email")
    }

    @Test
    fun malformedSurrogateUnicodeIsRejectedWithoutReplacingIt() {
        val malformed = "prefix\uD800suffix"
        assertIssue(validateRegistrationForm("person@example.test", malformed), "password", "invalid_password_unicode")
        assertIssue(validateLoginForm("person@example.test", malformed), "password", "invalid_password_unicode")
        assertIssue(validatePasswordResetForm("pr1_" + "a".repeat(64), malformed), "password", "invalid_password_unicode")
        assertTrue(validateRegistrationForm("person@example.test", "😀".repeat(15)).isLocallyValid)
    }

    @Test
    fun validationResultsAreRedactedAndAlwaysRequireServerChecks() {
        val submittedEmail = "private-marker"
        val submittedPassword = "password-secret-marker"
        val result = validateRegistrationForm(submittedEmail, submittedPassword)

        assertFalse(result.isLocallyValid)
        assertTrue(result.serverChecksRequired)
        assertFalse(result.toString().contains(submittedEmail))
        assertFalse(result.toString().contains(submittedPassword))
        assertTrue(result.issues.all { it.field != submittedEmail && it.code != submittedPassword })
        assertEquals("email", result.issues.first().field)
    }

    private fun assertIssue(result: FormValidation, field: String, code: String) {
        assertFalse(result.isLocallyValid)
        assertTrue(result.serverChecksRequired)
        assertTrue(result.issues.contains(ValidationIssue(field, code)), "Expected $field/$code, got ${result.issues}")
    }
}
