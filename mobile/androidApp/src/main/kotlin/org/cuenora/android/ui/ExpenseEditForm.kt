package org.cuenora.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import org.cuenora.shared.domain.Transaction
import org.cuenora.shared.repository.CatalogCategory
import org.cuenora.shared.repository.TransactionPatch
import org.cuenora.shared.validation.expenseEditPatch

@Composable
internal fun ExpenseEditForm(
    expense: Transaction,
    categories: List<CatalogCategory>,
    loadingCategories: Boolean,
    categoryFailure: Boolean,
    busy: Boolean,
    notice: String?,
    onLoadCategories: () -> Unit,
    back: () -> Unit,
    save: (TransactionPatch) -> Unit,
) {
    var amount by remember { mutableStateOf(accountAmount(expense.amount).replace(",", "")) }
    var date by remember { mutableStateOf(with(expense.occurredOn) {
        "${year.toString().padStart(4, '0')}-${month.toString().padStart(2, '0')}-${day.toString().padStart(2, '0')}"
    }) }
    var category by remember { mutableStateOf(expense.categoryId) }
    var description by remember { mutableStateOf(expense.description.orEmpty()) }
    var categoryOpen by remember { mutableStateOf(false) }
    var review by remember { mutableStateOf<TransactionPatch?>(null) }
    var validation by remember { mutableStateOf(false) }
    val categoryLabel = categories.firstOrNull { it.id == category }?.name
        ?: if (category == null) "No category" else "Current category"
    Column(Modifier.fillMaxSize().testTag("expense-edit-form").verticalScroll(rememberScrollState())
        .padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        TextButton(onClick = { if (review == null) back() else review = null }, enabled = !busy) {
            Text(if (review == null) "← Transaction details" else "← Keep editing")
        }
        Text(if (review == null) "Edit expense" else "Review expense correction",
            style = MaterialTheme.typography.headlineMedium)
        Text("Currency: ${expense.amount.currency.name}. Account assignment stays the same.")
        val pending = review
        if (pending == null) {
            CuenoraField("Amount", amount, { amount = it }, Modifier.testTag("expense-edit-amount"),
                decimal = true, canonicalizeDecimal = true)
            CuenoraDateField("Business date", date, { date = it }, Modifier.testTag("expense-edit-date"))
            Box {
                OutlinedButton(onClick = { categoryOpen = true }, enabled = !loadingCategories && !categoryFailure) {
                    Text("Category: $categoryLabel")
                }
                DropdownMenu(expanded = categoryOpen, onDismissRequest = { categoryOpen = false }) {
                    DropdownMenuItem(text = { Text("No category") }, onClick = { category = null; categoryOpen = false })
                    categories.forEach { option ->
                        DropdownMenuItem(text = { Text(option.name) },
                            onClick = { category = option.id; categoryOpen = false })
                    }
                }
            }
            if (categoryFailure) TextButton(onClick = onLoadCategories) { Text("Reload categories") }
            CuenoraField("Description", description, { description = it }, Modifier.testTag("expense-edit-description"))
            if (validation) Text("Enter a positive amount, a valid business date, and at least one change.")
            CuenoraAction("Review changes", {
                review = expenseEditPatch(expense, amount, date, category, description.ifEmpty { null })
                validation = review == null
            }, Modifier.testTag("expense-edit-review"))
        } else {
            CuenoraAmount("Expense", expense.amount.currency.name, amount)
            Text("Business date: $date")
            Text("Category: $categoryLabel")
            Text("Description: ${description.ifEmpty { "None" }}")
            Text("This corrects the recorded expense without moving money at a bank.")
            CuenoraAction("Save changes", { save(pending) }, Modifier.testTag("expense-edit-save"), busy = busy)
        }
        notice?.let { Text(it) }
    }
}
