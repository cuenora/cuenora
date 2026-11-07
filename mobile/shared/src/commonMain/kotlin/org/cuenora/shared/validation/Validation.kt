package org.cuenora.shared.validation

data class ValidationIssue(val field: String, val code: String)

data class FormValidation(val issues: List<ValidationIssue>) {
    val isLocallyValid: Boolean
        get() = issues.isEmpty()

    val serverChecksRequired: Boolean
        get() = true
}
