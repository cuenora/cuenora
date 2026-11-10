package org.cuenora.android

import androidx.compose.ui.test.*
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.test.junit4.v2.createComposeRule
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.savings.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class SavingsTest {
    @get:Rule val compose = createComposeRule()
    private val account = Account(AccountId(0, 1), OwnerId(0, 2), AccountName("Savings"), AccountType.SAVINGS, Currency.SEK, AccountStatus.ACTIVE)
    private val actions = mutableListOf<SavingsIntent>()
    private fun render(state: SavingsState) {
        compose.setContent { CuenoraTheme { SavingsPanel(state, { actions += it }, {}) } }
    }
    private fun click(text: String) = compose.onNodeWithText(text).performScrollTo().performClick()

    @Test fun datesStartEmptyAndForecastRequiresExplicitAction() {
        render(SavingsState(SavingsStage.Browsing, account, receivingAccounts = listOf(account)))
        click("Forecast")
        compose.onNodeWithTag("savings-start").assert(SemanticsMatcher.expectValue(SemanticsProperties.EditableText, AnnotatedString("")))
        compose.onNodeWithTag("savings-end").assert(SemanticsMatcher.expectValue(SemanticsProperties.EditableText, AnnotatedString("")))
        assertTrue(actions.isEmpty())
        compose.onNodeWithTag("savings-start").performTextInput("2099-01-01")
        compose.onNodeWithTag("savings-end").performTextInput("2099-01-31")
        click("Calculate estimate")
        assertEquals(listOf(SavingsIntent.Forecast("2099-01-01", "2099-01-31")), actions)
    }
    @Test fun actualReceiptReviewShowsOnlyReceivingAccountEffectAndNeedsConfirmation() {
        render(SavingsState(SavingsStage.Review, account, action = SavingsAction.Receipt(AccountId(0, 3), "4.00", Currency.SEK, "2026-11-10")))
        compose.onNodeWithText("Actual credited interest: 4.00 SEK").assertExists()
        compose.onNodeWithText("Only the receiving account is credited. No second credit is added to savings.").assertExists()
        assertTrue(actions.isEmpty())
        click("Confirm actual receipt")
        assertEquals(listOf(SavingsIntent.Confirm), actions)
    }
    @Test fun uncertainWriteOffersOriginalRetryAndNoNewForms() {
        render(SavingsState(SavingsStage.Uncertain, account, action = SavingsAction.Rate(account.id,
            SavingsRateCommand(BusinessDate(2026, 11, 10), "3.00"))))
        compose.onNodeWithText("Add rate version").assertDoesNotExist()
        click("Retry original request")
        assertEquals(listOf(SavingsIntent.Confirm), actions)
    }
    @Test fun termsRequireExplicitCapitalizationChoice() {
        render(SavingsState(SavingsStage.Browsing, account))
        click("Terms and rates")
        compose.onNodeWithTag("savings-effective").performTextInput("2026-11-10")
        click("Monthly")
        click("Review terms")
        assertTrue(actions.isEmpty())
        click("Exclude projected payouts")
        click("Review terms")
        assertEquals(SavingsIntent.Review(SavingsAction.Terms(account.id, SavingsTermsCommand(BusinessDate(2026, 11, 10), "monthly", false))), actions.single())
    }
    @Test fun arbitraryPrecisionProjectionIsLabeledAndNotAnActualReceipt() {
        val estimate = SavingsEstimate(account.id, Currency.SEK, BusinessDate(2099, 1, 1), BusinessDate(2099, 1, 31), BusinessDate(2026, 11, 10),
            Money(100000, Currency.SEK), "constant_posted_snapshot", false, true,
            SavingsTermsCommand(BusinessDate(2099, 1, 1), "monthly", false), emptyList(),
            listOf(SavingsEstimatePeriod(BusinessDate(2099, 1, 1), BusinessDate(2099, 1, 31), ExactAccrual("999999999999999999999", "7"), "99999999999999999999.00")))
        render(SavingsState(SavingsStage.Browsing, account, estimate = estimate))
        click("Forecast")
        compose.onNodeWithText("Estimate only — no balance or realized income changes.").performScrollTo().assertExists()
        compose.onNodeWithText("Projected payout: 99999999999999999999.00 SEK").performScrollTo().assertExists()
        compose.onNodeWithText("Confirm actual receipt").assertDoesNotExist()
    }
}
