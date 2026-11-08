package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.AccountBox
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationDrawerItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import kotlinx.coroutines.launch
import org.cuenora.shared.SharedModule

internal enum class Destination(val label: String, val icon: ImageVector) {
    Home("Home", Icons.Default.Home),
    Accounts("Accounts", Icons.Default.AccountBox),
    Transactions("Transactions", Icons.AutoMirrored.Filled.List),
    Insights("Insights", Icons.Default.Info),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CuenoraApp() {
    val nav = rememberNavController()
    val entry by nav.currentBackStackEntryAsState()
    val route = entry?.destination?.route ?: Destination.Home.name
    val selected = Destination.entries.firstOrNull { it.name == route }
    val drawer = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    val fontScale = LocalDensity.current.fontScale
    BoxWithConstraints(Modifier.fillMaxSize()) {
        // Reserve enough room for the full IBM Plex Mono destination labels.
        val useList = maxWidth < 440.dp || fontScale > 1.2f
        val useRail = maxWidth >= 600.dp && !useList
        fun navigate(destination: Destination) {
            nav.navigate(destination.name) {
                popUpTo(nav.graph.findStartDestination().id) { saveState = true }
                launchSingleTop = true
                restoreState = true
            }
        }
        ModalNavigationDrawer(
            drawerState = drawer, gesturesEnabled = useList,
            drawerContent = {
                if (useList) ModalDrawerSheet(modifier = Modifier.testTag("destination-list")) {
                    Column(Modifier.verticalScroll(rememberScrollState()).padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text("Destinations", style = MaterialTheme.typography.headlineSmall,
                            modifier = Modifier.semantics { heading() })
                        Destination.entries.forEach { destination ->
                            NavigationDrawerItem(
                                modifier = Modifier.testTag("nav-${destination.name}")
                                    .heightIn(min = (56 * fontScale).dp).cuenoraFocus(),
                                label = { Text(destination.label) },
                                icon = { Icon(destination.icon, contentDescription = null) },
                                selected = selected == destination,
                                onClick = { navigate(destination); scope.launch { drawer.close() } },
                            )
                        }
                    }
                }
            },
        ) {
            Row {
                if (useRail) {
                    NavigationRail(modifier = Modifier.testTag("navigation-rail")) {
                        Destination.entries.forEach { destination ->
                            NavigationRailItem(
                                modifier = Modifier.testTag("nav-${destination.name}").cuenoraFocus(),
                                selected = selected == destination, onClick = { navigate(destination) },
                                icon = { Icon(destination.icon, contentDescription = null) },
                                label = { Text(destination.label, style = MaterialTheme.typography.labelSmall) },
                            )
                        }
                    }
                }
                Scaffold(
                    topBar = {
                        TopAppBar(
                            title = { Text(selected?.label ?: "Base components",
                                modifier = Modifier.testTag("destination-title").semantics { heading() }) },
                            navigationIcon = {
                                if (selected == null) {
                                    IconButton(onClick = { nav.popBackStack() }) {
                                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                                    }
                                } else if (useList) {
                                    IconButton(onClick = { scope.launch { drawer.open() } }) {
                                        Icon(Icons.Default.Menu, "Open destinations")
                                    }
                                }
                            },
                        )
                    },
                    bottomBar = {
                        if (!useList && !useRail && selected != null) {
                            NavigationBar(modifier = Modifier.testTag("navigation-bar")) {
                                Destination.entries.forEach { destination ->
                                    NavigationBarItem(
                                        modifier = Modifier.testTag("nav-${destination.name}").cuenoraFocus(),
                                        selected = selected == destination,
                                        onClick = { navigate(destination) },
                                        icon = { Icon(destination.icon, contentDescription = null) },
                                        label = { Text(destination.label, style = MaterialTheme.typography.labelSmall) },
                                    )
                                }
                            }
                        }
                    },
                ) { padding ->
                    NavHost(
                        navController = nav, startDestination = Destination.Home.name,
                        modifier = Modifier.padding(padding).fillMaxSize(),
                        enterTransition = { EnterTransition.None }, exitTransition = { ExitTransition.None },
                        popEnterTransition = { EnterTransition.None }, popExitTransition = { ExitTransition.None },
                    ) {
                        Destination.entries.forEach { destination ->
                            composable(destination.name) {
                                FoundationDestination(destination) { nav.navigate("components") { launchSingleTop = true } }
                            }
                        }
                        composable("components") { ComponentSpecimens() }
                    }
                }
            }
        }
        BackHandler(enabled = drawer.isOpen) { scope.launch { drawer.close() } }
    }
}

@Composable
private fun FoundationDestination(destination: Destination, onComponents: () -> Unit) {
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(24.dp),
    ) {
        Text("Cuenora", style = MaterialTheme.typography.displayLarge)
        CuenoraNotice(
            "Android foundation",
            "Navigation, theme and reusable controls. Financial screens are not connected yet.",
        )
        if (destination == Destination.Home) {
            Text("Made for Android", style = MaterialTheme.typography.headlineSmall)
            Text("A native foundation for clear financial records. No financial data is loaded or saved here.")
            TextButton(onClick = onComponents, modifier = Modifier.cuenoraFocus()) { Text("View base components") }
        } else {
            Text("${destination.label} will be connected in a later roadmap task.")
        }
        Text(SharedModule.name, style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun ComponentSpecimens() {
    var name by rememberSaveable { mutableStateOf("Everyday") }
    var showDiscard by rememberSaveable { mutableStateOf(false) }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(24.dp),
    ) {
        Text("Component specimens", style = MaterialTheme.typography.headlineMedium)
        Text("Synthetic examples only. Nothing is sent or saved.")
        CuenoraField("Account name", name, { name = it }, supportingText = "Choose a recognizable name.")
        CuenoraAction("Continue", {}, unavailableReason = "This preview does not submit records.")
        CuenoraAmount("Recorded · specimen", "SEK", "+1,250.00")
        CuenoraStatus("Planned · No balance effect")
        CuenoraNotice("Unavailable read", "Current data is unavailable. No balance is shown.", NoticeTone.Error)
        TextButton(onClick = { showDiscard = true }) { Text("Preview discard dialog") }
    }
    if (showDiscard) CuenoraDiscardDialog(
        onKeepEditing = { showDiscard = false },
        onDiscard = { name = ""; showDiscard = false },
    )
}
